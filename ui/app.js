// The connection screen: choose your Marea, check it and enter; or, when it doesn't answer, wait and retry.
'use strict';

const { invoke } = window.__TAURI__.core;
const $ = (selector) => document.querySelector(selector);

const form = $('#setup-form');
const address = $('#address');
const setupError = $('#setup-error');
const connectButton = $('#connect');
const backButton = $('#back');
const retryButton = $('#retry');

let strings = {}; // texts in the system language, provided by the app
let saved = null; // the saved server ("marea.example.com"), if any
let timer = null;
let failures = 0; // retries in a row without success (to wait a little longer each time)

/** A translated text, with {placeholders} filled in. */
function t(key, values) {
  let text = strings[key] || key;
  for (const [name, value] of Object.entries(values || {})) text = text.split('{' + name + '}').join(String(value));
  return text;
}

function translatePage(lang) {
  document.documentElement.lang = lang;
  for (const el of document.querySelectorAll('[data-i18n]')) el.textContent = t(el.dataset.i18n);
  for (const el of document.querySelectorAll('[data-i18n-placeholder]')) el.placeholder = t(el.dataset.i18nPlaceholder);
}

function show(name) {
  for (const view of document.querySelectorAll('[data-view]')) view.hidden = view.dataset.view !== name;
}

function stopTimer() {
  if (timer) clearInterval(timer);
  timer = null;
}

function setError(message) {
  setupError.textContent = message || '';
  setupError.hidden = !message;
  if (message) address.setAttribute('aria-invalid', 'true');
  else address.removeAttribute('aria-invalid');
}

function apply(state) {
  if (state.mode === 'connecting') connecting(state.name, state.host);
  else if (state.mode === 'offline') offline(state.host, state.message);
  else setup(null);
}

function setup(current) {
  stopTimer();
  show('setup');
  address.value = current || '';
  backButton.hidden = !saved;
  backButton.textContent = saved ? t('setup.back', { host: saved }) : '';
  setError('');
  connectButton.disabled = false;
  connectButton.textContent = t('setup.connect');
  setTimeout(() => { address.focus(); address.select(); }, 0);
}

function connecting(name, host) {
  stopTimer();
  failures = 0;
  show('connecting');
  $('#connecting-title').textContent = t('connecting.title', { name: name || t('connecting.yours') });
  $('#connecting-host').textContent = host || '';
}

function offline(host, message) {
  stopTimer();
  show('offline');
  $('#offline-host').textContent = host || '';
  $('#offline-message').textContent = message || '';
  retryButton.disabled = false;
  retryButton.textContent = t('offline.retry');
  // 10 s, 20 s, 30 s… up to a minute between attempts
  let left = Math.min(60, 10 * (failures + 1));
  const countdown = $('#offline-countdown');
  const paint = () => { countdown.textContent = t('offline.countdown', { seconds: left }); };
  paint();
  timer = setInterval(() => {
    left -= 1;
    if (left <= 0) retry();
    else paint();
  }, 1000);
}

async function retry() {
  stopTimer();
  failures += 1;
  retryButton.disabled = true;
  retryButton.textContent = t('offline.retrying');
  $('#offline-countdown').textContent = '';
  try {
    apply(await invoke('retry'));
  } catch (e) {
    offline(saved, String(e));
  }
}

form.addEventListener('submit', async (ev) => {
  ev.preventDefault();
  setError('');
  connectButton.disabled = true;
  backButton.disabled = true;
  connectButton.textContent = t('setup.checking');
  try {
    const info = await invoke('connect', { address: address.value });
    saved = info.host;
    connecting(info.name, info.host);
  } catch (e) {
    setError(String(e));
    connectButton.disabled = false;
    connectButton.textContent = t('setup.connect');
    address.focus();
  } finally {
    backButton.disabled = false;
  }
});

address.addEventListener('input', () => setError(''));

backButton.addEventListener('click', async () => {
  try {
    await invoke('cancel_setup');
  } catch (e) {
    setError(String(e));
  }
});

$('#change').addEventListener('click', () => setup(saved));
retryButton.addEventListener('click', retry);

// As soon as the network is back there's no need to wait for the countdown
window.addEventListener('online', () => {
  if (!$('[data-view="offline"]').hidden) retry();
});

async function boot() {
  try {
    const info = await invoke('app_info');
    strings = info.strings || {};
    saved = info.server;
    translatePage(info.lang);
    $('#footer').textContent = t('footer', { version: info.version });
    if (new URLSearchParams(location.search).has('change')) {
      setup(saved);
      return;
    }
    show('loading');
    apply(await invoke('start'));
  } catch (e) {
    setup(null);
    setError(String(e));
  }
}

boot();
