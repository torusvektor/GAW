// Layout choice for the hosted web build (see src/web-main.ts): the full
// editor or the touch-first mobile surface. Shared so the mobile picker and
// the editor's "Mobile layout" button can switch it.

export type WebUi = 'desktop' | 'mobile';

const UI_KEY = 'ga-web-ui';

/** True when running from the hosted web build rather than the desktop
 *  app, its LAN companion server or the native mobile shell. */
export function isHostedWeb(): boolean {
  return typeof document !== 'undefined' && !!document.documentElement.dataset.webUi;
}

export function readStoredUi(): WebUi | null {
  try {
    const raw = localStorage.getItem(UI_KEY);
    return raw === 'desktop' || raw === 'mobile' ? raw : null;
  } catch {
    return null;
  }
}

export function storeUi(ui: WebUi | null): void {
  try {
    if (ui) localStorage.setItem(UI_KEY, ui);
    else localStorage.removeItem(UI_KEY);
  } catch { /* private mode */ }
}

/** Switches layout and reloads; drops any ?ui= override from the URL so it
 *  does not immediately switch back. */
export function switchWebUi(ui: WebUi): void {
  storeUi(ui);
  const url = new URL(window.location.href);
  url.searchParams.delete('ui');
  window.location.replace(url.toString());
}
