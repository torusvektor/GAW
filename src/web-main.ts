// Entry point for the hosted browser build (`npm run build:web`).
//
// One URL serves every device: laptops and desktops (MacBook, PC) get the
// full editor from main.ts, phones and small tablets get the touch-first
// standalone VJ surface from native-mobile-main.ts. Both modules boot
// themselves on import, so this file only decides which one to load.
//
// Overrides: `?ui=desktop` or `?ui=mobile` forces a layout and is
// remembered; `?ui=auto` clears the remembered choice.

import { readStoredUi, storeUi, switchWebUi, type WebUi } from './lib/webUi';

const MOBILE_MODES = new Set(['mobile-standalone', 'standalone', 'mobile-remote', 'remote']);

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

// The full editor on a touch device gets a way back to the mobile layout;
// the editor itself has no such control.
function addMobileLayoutButton() {
  const button = document.createElement('button');
  button.type = 'button';
  button.textContent = 'Mobile layout';
  button.setAttribute('aria-label', 'Switch to the mobile layout');
  button.style.cssText = [
    'position:fixed', 'z-index:2147483000',
    'right:calc(12px + env(safe-area-inset-right, 0px))',
    'bottom:calc(44px + env(safe-area-inset-bottom, 0px))',
    'padding:8px 12px', 'border-radius:999px', 'border:1px solid rgba(187,134,252,.6)',
    'background:rgba(10,10,12,.85)', 'color:#e8e8ee', 'font:600 13px -apple-system,system-ui,sans-serif',
  ].join(';');
  button.addEventListener('click', () => switchWebUi('mobile'));
  document.body.appendChild(button);
}

const ui = document.documentElement.dataset.webUi as WebUi;
const isPopup = params.has('mode');
if (ui === 'mobile') {
  void import('./native-mobile-main');
} else {
  void import('./main');
  if (!isPopup && looksLikeTouchDevice()) addMobileLayoutButton();
}
