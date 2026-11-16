//! Shortcuts to Marea's own sections and actions, used by the macOS menu bar and the menu of the icon next to the
//! clock (Windows and Linux): the same shortcuts the web app offers when installed from a browser, and more.
//!
//! Each one runs a fixed line of JavaScript, written here, in your Marea's page: it changes the page's address
//! (`#/mail`, `#/agenda`…) or presses one of Marea's own keys. Nothing from outside ever reaches these scripts.

/// Menu ids. All of them start with `marea:` so `on_menu` can tell them apart.
pub const NEW_WAVE: &str = "marea:new_wave";
pub const COMPOSE: &str = "marea:compose";
pub const SEARCH: &str = "marea:search";
pub const ASSISTANT: &str = "marea:assistant";
pub const SETTINGS: &str = "marea:settings";
pub const SHORTCUTS: &str = "marea:shortcuts";
pub const WAVES: &str = "marea:waves";
pub const MAIL: &str = "marea:mail";
pub const AGENDA: &str = "marea:agenda";
pub const PEOPLE: &str = "marea:people";
pub const DROPS: &str = "marea:drops";
pub const PODCASTS: &str = "marea:podcasts";
pub const NOTES: &str = "marea:notes";

/// The sections, in the order of the "Go" menu (⌘1 to ⌘7): menu id and locale key. On Windows and Linux, the
/// bridge has the same list for Ctrl+1 to Ctrl+7 (a test checks they match).
pub const SECTIONS: [(&str, &str); 7] = [
    (WAVES, "menu.waves"),
    (MAIL, "menu.mail"),
    (AGENDA, "menu.agenda"),
    (PEOPLE, "menu.people"),
    (DROPS, "menu.drops"),
    (PODCASTS, "menu.podcasts"),
    (NOTES, "menu.notes"),
];

/// Presses one of Marea's keys on the page body, as if typed outside any text box (Marea ignores keys typed in
/// text boxes, and ⌘K inside an editor means "link").
macro_rules! press {
    ($init:literal) => {
        concat!("document.body.dispatchEvent(new KeyboardEvent('keydown',", $init, "))")
    };
}

/// The script for a menu id, or `None` if the id isn't one of these shortcuts.
pub fn script(id: &str) -> Option<&'static str> {
    Some(match id {
        NEW_WAVE => "location.hash='#/new'",
        COMPOSE => "location.hash='#/mail/new'",
        SEARCH => "(function(){var s=document.querySelector('input.search');if(s){s.focus();s.select();}})()",
        ASSISTANT => press!("{key:'k',ctrlKey:true,metaKey:true,bubbles:true}"),
        SHORTCUTS => press!("{key:'?',bubbles:true}"),
        SETTINGS => "location.hash='#/settings'",
        WAVES => "location.hash='#/'",
        MAIL => "location.hash='#/mail'",
        AGENDA => "location.hash='#/agenda'",
        PEOPLE => "location.hash='#/people'",
        DROPS => "location.hash='#/drop'",
        PODCASTS => "location.hash='#/podcasts'",
        NOTES => "location.hash='#/notas'",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_section_has_a_script() {
        for (id, _) in SECTIONS {
            assert!(script(id).is_some_and(|js| js.starts_with("location.hash='#/")), "{id}");
        }
        for id in [NEW_WAVE, COMPOSE, SEARCH, ASSISTANT, SHORTCUTS, SETTINGS] {
            assert!(script(id).is_some(), "{id}");
        }
        assert!(script("quit").is_none());
    }

    #[test]
    fn the_bridge_has_the_same_sections() {
        let bridge = include_str!("bridge.js");
        let start = bridge.find("var SECTIONS = [").expect("SECTIONS in bridge.js") + "var SECTIONS = [".len();
        let list = &bridge[start..start + bridge[start..].find(']').expect("end of SECTIONS")];
        let from_bridge: Vec<&str> = list.split(',').map(|item| item.trim().trim_matches('\'')).collect();
        let from_menu: Vec<&str> = SECTIONS
            .iter()
            .map(|(id, _)| script(id).unwrap().trim_start_matches("location.hash='").trim_end_matches('\''))
            .collect();
        assert_eq!(from_bridge, from_menu);
    }

    #[test]
    fn keys_go_to_the_page_body() {
        assert_eq!(
            script(ASSISTANT),
            Some("document.body.dispatchEvent(new KeyboardEvent('keydown',{key:'k',ctrlKey:true,metaKey:true,bubbles:true}))")
        );
    }
}
