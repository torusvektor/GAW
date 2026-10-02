// Entry point for the hosted browser build (`npm run build:web`).
//
// One URL serves every device: laptops and desktops (MacBook, PC) get the
// full editor from main.ts, phones and small tablets get the touch-first
// standalone VJ surface from native-mobile-main.ts. Both modules boot
// themselves on import, so this file only decides which one to load.
//
// Overrides: `?ui=desktop` or `?ui=mobile` forces a layout and is
// remembered; `?ui=auto` clears the remembered choice.

const UI_KEY = 'ga-web-ui';
type WebUi = 'desktop' | 'mobile';

const MOBILE_MODES = new Set(['mobile-standalone', 'standalone', 'mobile-remote', 'remote']);

function readStoredUi(): WebUi | null {
  try {
    const raw = localStorage.getItem(UI_KEY);
    return raw === 'desktop' || raw === 'mobile' ? raw : null;
  } catch {
    return null;
  }
}

function storeUi(ui: WebUi | null) {
  try {
    if (ui) localStorage.setItem(UI_KEY, ui);
    else localStorage.removeItem(UI_KEY);
  } catch { /* private mode */ }
}

function looksLikeTouchDevice(): boolean {
  const ua = navigator.userAgent;
  if (/iPhone|iPod|Android.+Mobile|Windows Phone/i.test(ua)) return true;
  // Tablets: touch-only pointer and a narrow screen. Large tablets such as
  // a 12.9" iPad Pro (1024 CSS px short edge) keep the full editor.
  const coarseOnly = matchMedia('(pointer: coarse)').matches && !matchMedia('(any-pointer: fine)').matches;
  const shortEdge = Math.min(screen.width, screen.height);
  return coarseOnly && shortEdge < 1024;
}

function pickUi(params: URLSearchParams): WebUi {
  const mode = params.get('mode');
  // Output / slice / stage windows are opened by the editor with ?mode=…
  // and must always run through main.ts.
  if (mode && !MOBILE_MODES.has(mode)) return 'desktop';
  if (mode && MOBILE_MODES.has(mode)) return 'mobile';

  const forced = params.get('ui');
  if (forced === 'desktop' || forced === 'mobile') {
    storeUi(forced);
    return forced;
  }
  if (forced === 'auto') storeUi(null);

  return readStoredUi() ?? (looksLikeTouchDevice() ? 'mobile' : 'desktop');
}

function registerServiceWorker() {
  if (!('serviceWorker' in navigator) || !window.isSecureContext) return;
  if (import.meta.env.DEV) return;
  // Only the top-level editor/mobile page registers; popup output windows
  // share the same scope and pick the worker up automatically.
  if (new URLSearchParams(window.location.search).has('mode')) return;
  window.addEventListener('load', () => {
    navigator.serviceWorker.register('./web-sw.js').catch((err) => {
      console.warn('[web] service worker registration failed:', err);
    });
  });
}

const params = new URLSearchParams(window.location.search);
document.documentElement.dataset.webUi = pickUi(params);
registerServiceWorker();

if (document.documentElement.dataset.webUi === 'mobile') {
  void import('./native-mobile-main');
} else {
  void import('./main');
}
