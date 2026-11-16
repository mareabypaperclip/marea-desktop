//! Your location, on macOS. Inside apps, Apple's web engine (WKWebView) never gives pages your location, and Tauri
//! has no way to allow it, so the bridge replaces `navigator.geolocation` and asks the app, which asks Core
//! Location. The first time, macOS shows its own "Marea would like to use your location" prompt (Info.plist has
//! the reason, in each language). Only the chosen Marea can ask, only when it asks (today, when you press "Use
//! my location" for the weather), and only at kilometre accuracy. On Windows and Linux the web engine does it.

use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Runtime};
use tokio::sync::oneshot;

/// A position, like the browser's `GeolocationCoordinates`.
#[derive(Debug, Serialize)]
pub struct Position {
    pub latitude: f64,
    pub longitude: f64,
    /// Metres.
    pub accuracy: f64,
}

/// Why there is no position: "denied", "unavailable" or "timeout" (the bridge turns them into the web's codes).
pub type Failure = &'static str;

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
type Reply = oneshot::Sender<Result<Position, Failure>>;

/// Long enough to read and answer the system's permission prompt the first time.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
const WAIT: Duration = Duration::from_secs(60);

/// Asks the system for one position.
#[cfg(target_os = "macos")]
pub async fn locate<R: Runtime>(app: &AppHandle<R>) -> Result<Position, Failure> {
    let (reply, answer) = oneshot::channel();
    app.run_on_main_thread(move || mac::start(reply)).map_err(|_| "unavailable")?;
    let result = match tokio::time::timeout(WAIT, answer).await {
        Ok(Ok(result)) => result,
        Ok(Err(_)) => Err("unavailable"),
        Err(_) => Err("timeout"),
    };
    let _ = app.run_on_main_thread(mac::stop);
    result
}

/// Elsewhere the web engine answers by itself, so the bridge never asks.
#[cfg(not(target_os = "macos"))]
pub async fn locate<R: Runtime>(_app: &AppHandle<R>) -> Result<Position, Failure> {
    Err("unavailable")
}

/// Core Location through plain Objective-C messages (no extra framework crates). Type-checked against objc2's
/// GNUstep runtime on Linux and compiled for real by the macOS check in CI.
#[cfg(target_os = "macos")]
mod mac {
    use std::cell::{Cell, RefCell};

    use objc2::encode::{Encode, Encoding};
    use objc2::rc::Retained;
    use objc2::runtime::{AnyClass, AnyObject, NSObject};
    use objc2::{define_class, msg_send, DefinedClass, MainThreadMarker, MainThreadOnly};

    use super::{Position, Reply};

    // The framework has to be linked for its classes to exist at run time.
    #[link(name = "CoreLocation", kind = "framework")]
    extern "C" {}

    /// `CLLocationCoordinate2D`.
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Coordinate {
        latitude: f64,
        longitude: f64,
    }

    // SAFETY: same layout and name as CLLocationCoordinate2D (two doubles).
    unsafe impl Encode for Coordinate {
        const ENCODING: Encoding = Encoding::Struct("CLLocationCoordinate2D", &[f64::ENCODING, f64::ENCODING]);
    }

    // CLAuthorizationStatus values.
    const NOT_DETERMINED: i32 = 0;
    const RESTRICTED: i32 = 1;
    const DENIED: i32 = 2;
    // CLError codes.
    const LOCATION_UNKNOWN: isize = 0;
    const ERROR_DENIED: isize = 1;
    /// kCLLocationAccuracyKilometer: enough for the weather, and the least revealing.
    const KILOMETRE: f64 = 1000.0;

    pub struct Ivars {
        reply: RefCell<Option<Reply>>,
        asked: Cell<bool>,
    }

    define_class!(
        // SAFETY: NSObject has no subclassing requirements, and Delegate doesn't implement Drop.
        #[unsafe(super(NSObject))]
        #[thread_kind = MainThreadOnly]
        #[name = "MareaLocationDelegate"]
        #[ivars = Ivars]
        pub struct Delegate;

        // The CLLocationManagerDelegate methods Marea needs. Core Location checks them with respondsToSelector:.
        impl Delegate {
            #[unsafe(method(locationManager:didUpdateLocations:))]
            fn did_update(&self, _manager: &AnyObject, locations: &AnyObject) {
                let last: Option<Retained<AnyObject>> = unsafe { msg_send![locations, lastObject] };
                if let Some(location) = last {
                    let at: Coordinate = unsafe { msg_send![&*location, coordinate] };
                    let accuracy: f64 = unsafe { msg_send![&*location, horizontalAccuracy] };
                    self.finish(Ok(Position { latitude: at.latitude, longitude: at.longitude, accuracy }));
                }
            }

            #[unsafe(method(locationManager:didFailWithError:))]
            fn did_fail(&self, _manager: &AnyObject, error: &AnyObject) {
                let code: isize = unsafe { msg_send![error, code] };
                match code {
                    // Temporary: Core Location keeps trying, and the command has its own time limit.
                    LOCATION_UNKNOWN => {}
                    ERROR_DENIED => self.finish(Err("denied")),
                    _ => self.finish(Err("unavailable")),
                }
            }

            #[unsafe(method(locationManagerDidChangeAuthorization:))]
            fn did_change_authorization(&self, manager: &AnyObject) {
                self.step(manager);
            }
        }
    );

    impl Delegate {
        fn new(mtm: MainThreadMarker, reply: Reply) -> Retained<Self> {
            let this = Self::alloc(mtm).set_ivars(Ivars { reply: RefCell::new(Some(reply)), asked: Cell::new(false) });
            unsafe { msg_send![super(this), init] }
        }

        /// Asks for permission the first time, then for one position.
        fn step(&self, manager: &AnyObject) {
            let status: i32 = unsafe { msg_send![manager, authorizationStatus] };
            match status {
                NOT_DETERMINED => unsafe { msg_send![manager, requestWhenInUseAuthorization] },
                RESTRICTED | DENIED => self.finish(Err("denied")),
                _ if !self.ivars().asked.replace(true) => unsafe { msg_send![manager, requestLocation] },
                _ => {}
            }
        }

        fn finish(&self, result: Result<Position, &'static str>) {
            if let Some(reply) = self.ivars().reply.borrow_mut().take() {
                let _ = reply.send(result);
            }
        }
    }

    thread_local! {
        /// The request in progress (one at a time), kept alive until it answers. Main thread only.
        static ACTIVE: RefCell<Option<(Retained<AnyObject>, Retained<Delegate>)>> = const { RefCell::new(None) };
    }

    /// Starts a request. Runs on the main thread.
    pub fn start(reply: Reply) {
        let (Some(mtm), Some(class)) = (MainThreadMarker::new(), AnyClass::get(c"CLLocationManager")) else {
            let _ = reply.send(Err("unavailable"));
            return;
        };
        let delegate = Delegate::new(mtm, reply);
        let manager: Retained<AnyObject> = unsafe { msg_send![class, new] };
        unsafe {
            let _: () = msg_send![&*manager, setDesiredAccuracy: KILOMETRE];
            let _: () = msg_send![&*manager, setDelegate: &*delegate];
        }
        delegate.step(&manager);
        ACTIVE.with(|active| *active.borrow_mut() = Some((manager, delegate)));
    }

    /// Lets the finished or abandoned request go. Runs on the main thread.
    pub fn stop() {
        ACTIVE.with(|active| {
            if let Some((manager, _delegate)) = active.borrow_mut().take() {
                unsafe {
                    let _: () = msg_send![&*manager, setDelegate: None::<&AnyObject>];
                }
            }
        });
    }
}
