// Compact layout for the full editor on phones and narrow windows (hosted web
// build only). The editor's desktop layout puts the toolbar, both sidebars
// and the canvas side by side, which does not fit a phone. Compact mode shows
// one region at a time, full screen, and switches between them from a bottom
// tab bar, the way the Standalone surface does. Inside VJ a second bar picks
// the clip grid, the media library or the effects.
//
// It works from the outside: a class on <html> plus CSS keyed to the
// editor's own region classes (.toolbar, .left-sidebar, .viewport,
// .right-sidebar, .statusbar, .vj-overlay …) and a click on the editor's own
// VJ button. App.svelte is not edited, so upstream releases keep merging.

export type CompactTab = 'canvas' | 'layers' | 'inspector' | 'vj' | 'tools';
export type VjSection = 'clips' | 'media' | 'effects';

const TABS: Array<{ id: CompactTab; label: string; icon: string }> = [
  { id: 'canvas', label: 'Canvas', icon: '<rect x="3" y="5" width="18" height="14" rx="2"/>' },
  { id: 'layers', label: 'Layers', icon: '<path d="M12 3 2 8l10 5 10-5-10-5Z"/><path d="m2 13 10 5 10-5"/>' },
  { id: 'inspector', label: 'Inspector', icon: '<path d="M4 6h10M18 6h2M4 12h4M12 12h8M4 18h12"/><circle cx="16" cy="6" r="2"/><circle cx="10" cy="12" r="2"/><circle cx="18" cy="18" r="2"/>' },
  { id: 'vj', label: 'VJ', icon: '<circle cx="7" cy="12" r="4"/><circle cx="17" cy="12" r="4"/><path d="M11 12h2"/>' },
  { id: 'tools', label: 'Tools', icon: '<path d="M4 7h16M4 12h16M4 17h16"/>' },
];

const VJ_SECTIONS: Array<{ id: VjSection; label: string }> = [
  { id: 'clips', label: 'Clips' },
  { id: 'media', label: 'Media' },
  { id: 'effects', label: 'Effects' },
];

const TAB_KEY = 'ga-compact-tab';
const VJ_KEY = 'ga-compact-vj';
const TAB_BAR = 58;
const SUB_BAR = 40;

const CSS = `
html.ga-compact, html.ga-compact body { height: 100%; overflow: hidden; }
html.ga-compact .app { height: calc(100dvh - ${TAB_BAR}px - env(safe-area-inset-bottom, 0px)) !important; }
/* The GPU advice banner takes half a phone screen. */
html.ga-compact .gpu-warning-banner { display: none !important; }
/* The floating button from web-main.ts moves into the Tools tab. */
html.ga-compact .ga-mobile-layout-btn { display: none !important; }
html.ga-compact header.toolbar,
html.ga-compact footer.statusbar { display: none !important; }

/* Canvas / Layers / Inspector: one region full screen. The viewport stays
   laid out underneath so the canvas keeps rendering at its real size. */
html.ga-compact main.main-content { position: relative; display: block !important; height: 100%; }
html.ga-compact .viewport { position: absolute !important; inset: 0; width: 100% !important; }
html.ga-compact .left-sidebar,
html.ga-compact .right-sidebar {
  display: none !important; position: absolute !important; inset: 0; z-index: 30;
  width: 100% !important; max-width: none !important; min-width: 0 !important;
  border: 0 !important; overflow-y: auto;
}
html.ga-compact[data-compact-tab="layers"] .left-sidebar,
html.ga-compact[data-compact-tab="inspector"] .right-sidebar { display: flex !important; }
html.ga-compact .left-sidebar > *, html.ga-compact .right-sidebar > *,
html.ga-compact .left-panel-host > *, html.ga-compact .right-sidebar > * > * {
  width: 100% !important; max-width: none !important;
}

/* Tools: the toolbar and the status bar's tray buttons as one sheet. */
html.ga-compact[data-compact-tab="tools"] header.toolbar {
  display: flex !important; flex-direction: column; align-items: stretch; justify-content: flex-start !important; gap: 14px;
  position: fixed; inset: 0 0 calc(${TAB_BAR}px + env(safe-area-inset-bottom, 0px)) 0;
  z-index: 40; height: auto !important; padding: calc(14px + env(safe-area-inset-top, 0px)) 14px 96px;
  overflow-y: auto; background: var(--ga-void, #070809);
}
html.ga-compact[data-compact-tab="tools"] header.toolbar > :is(.toolbar-left, .toolbar-center, .toolbar-right) {
  display: flex !important; flex: 0 0 auto !important; flex-wrap: wrap; align-items: center; justify-content: flex-start;
  gap: 8px; width: 100%; min-width: 0; height: auto !important; margin: 0 !important; position: static !important;
}
html.ga-compact[data-compact-tab="tools"] footer.statusbar {
  display: flex !important; flex-wrap: wrap; gap: 8px; position: fixed; left: 0; right: 0;
  bottom: calc(${TAB_BAR}px + env(safe-area-inset-bottom, 0px)); z-index: 41; height: auto !important;
  padding: 8px 14px; background: var(--ga-panel, #0b0d11);
}
html.ga-compact[data-compact-tab="tools"] footer.statusbar .spacer { display: none; }

/* VJ: the editor's own mixer overlay, hidden (not closed) on other tabs so
   live output keeps running, and split into Clips / Media / Effects. */
html.ga-compact:not([data-compact-tab="vj"]) .vj-overlay { display: none !important; }
html.ga-compact .vj-overlay { bottom: calc(${TAB_BAR + SUB_BAR}px + env(safe-area-inset-bottom, 0px)) !important; height: auto !important; }
/* The header is a three-column grid with the MIX / STAGE / MAP switch
   pinned in the middle; on a phone the columns overlap, so wrap it into
   rows: actions, mode switch, macro knobs. */
html.ga-compact .vj-header {
  display: flex !important; flex-wrap: wrap !important; align-items: center;
  gap: 6px !important; height: auto !important; min-height: 0 !important; padding: 6px 8px !important;
}
html.ga-compact .vj-header > * { position: static !important; transform: none !important; margin: 0 !important; }
html.ga-compact .vj-header .header-lead,
html.ga-compact .vj-header .header-left { flex: 0 1 auto; flex-wrap: wrap; min-width: 0; width: auto !important; }
html.ga-compact .vj-header .header-right { flex: 1 1 auto; flex-wrap: wrap; justify-content: flex-end; justify-self: auto; }
html.ga-compact .vj-header .header-stage-slot { order: 2; flex: 1 1 100%; display: flex; justify-content: center; }
html.ga-compact .vj-header .header-stage { position: static !important; transform: none !important; }
html.ga-compact .vj-header .header-macros {
  order: 3; flex: 1 1 100%; width: 100% !important; max-width: none !important;
  overflow-x: auto; justify-content: flex-start;
}
html.ga-compact .vj-main { display: block !important; position: relative; overflow: hidden; }
html.ga-compact .vj-main > * {
  position: absolute !important; inset: 0; width: 100% !important; height: 100% !important;
  max-width: none !important; display: none !important;
}
html.ga-compact .vj-main .right-panel-vj { width: 100% !important; }
html.ga-compact .vj-main .vj-right-tray-toggle { display: none !important; }
html.ga-compact .vj-main .vj-right-tray-content { width: 100% !important; display: block !important; }
html.ga-compact[data-compact-vj="clips"] .vj-main > .vj-bottom,
html.ga-compact[data-compact-vj="effects"] .vj-main > .vj-preview-section { display: flex !important; overflow: auto; }
/* Media lives inside the preview section's right panel; show only it. */
html.ga-compact[data-compact-vj="media"] .vj-main > .vj-preview-section { display: block !important; }
html.ga-compact[data-compact-vj="media"] .vj-preview-section > :not(.right-panel-vj) { display: none !important; }
html.ga-compact[data-compact-vj="media"] .vj-preview-section > .right-panel-vj { position: absolute !important; inset: 0; display: block !important; overflow: auto; }
html.ga-compact[data-compact-vj="effects"] .vj-preview-section > .right-panel-vj { display: none !important; }
html.ga-compact .vj-bottom .grid-section { width: 100%; overflow: auto; }

.ga-compact-tabs {
  position: fixed; left: 0; right: 0; bottom: 0; z-index: 2147482000;
  padding-bottom: env(safe-area-inset-bottom, 0px);
  background: rgba(10, 11, 14, 0.97); border-top: 1px solid rgba(255, 255, 255, 0.1);
  font: 600 11px/1 -apple-system, system-ui, 'Segoe UI', Roboto, sans-serif;
}
.ga-compact-main { display: grid; grid-template-columns: repeat(${TABS.length}, 1fr); height: ${TAB_BAR}px; }
.ga-compact-main button {
  display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 5px;
  background: none; border: 0; color: #8d93a6; letter-spacing: 0.02em; cursor: pointer;
  -webkit-tap-highlight-color: transparent;
}
.ga-compact-main button[aria-selected="true"] { color: #e6e8ef; }
.ga-compact-main button[aria-selected="true"] svg { stroke: #c18cff; }
.ga-compact-main svg { width: 22px; height: 22px; fill: none; stroke: currentColor; stroke-width: 1.8; stroke-linecap: round; stroke-linejoin: round; }
.ga-compact-sub { display: none; gap: 6px; height: ${SUB_BAR}px; padding: 6px 10px 0; }
html[data-compact-tab="vj"] .ga-compact-sub { display: flex; }
.ga-compact-sub button {
  flex: 1; border-radius: 8px; border: 1px solid rgba(255, 255, 255, 0.12); background: #15171d;
  color: #a7adbf; font: 600 12px/1 -apple-system, system-ui, sans-serif; letter-spacing: 0.04em; text-transform: uppercase;
}
.ga-compact-sub button[aria-selected="true"] { background: #2a1f3d; border-color: #c18cff; color: #f1e8ff; }
.ga-compact-tabs button:focus-visible { outline: 2px solid #c18cff; outline-offset: -3px; }
.ga-compact-switch {
  align-self: flex-start; padding: 10px 14px; border-radius: 999px; border: 1px solid rgba(193, 140, 255, 0.6);
  background: transparent; color: #e6e8ef; font: 600 13px -apple-system, system-ui, sans-serif;
}
`;

function read<T extends string>(key: string, allowed: readonly T[], fallback: T): T {
  try {
    const raw = localStorage.getItem(key);
    if (raw && (allowed as readonly string[]).includes(raw)) return raw as T;
  } catch { /* private mode */ }
  return fallback;
}

function store(key: string, value: string): void {
  try { localStorage.setItem(key, value); } catch { /* private mode */ }
}

/** The VJ mixer is the editor's own overlay, mounted while it is open. */
function vjOpen(): boolean {
  return !!document.querySelector('.vj-overlay');
}

function tabButton(id: string, html: string, onClick: () => void): HTMLButtonElement {
  const button = document.createElement('button');
  button.type = 'button';
  button.setAttribute('role', 'tab');
  button.dataset.id = id;
  button.innerHTML = html;
  button.addEventListener('click', onClick);
  return button;
}

export interface CompactEditorOptions {
  /** Shown in the Tools tab to leave the full editor. */
  onMobileLayout?: () => void;
}

/** Turns compact mode on while `query` matches and off when it stops
 *  matching (rotating a tablet, resizing a window). */
export function installCompactEditor(query: MediaQueryList, options: CompactEditorOptions = {}): void {
  const root = document.documentElement;
  const style = document.createElement('style');
  style.textContent = CSS;
  document.head.appendChild(style);

  const nav = document.createElement('nav');
  nav.className = 'ga-compact-tabs';
  nav.setAttribute('aria-label', 'Editor sections');
  const sub = document.createElement('div');
  sub.className = 'ga-compact-sub';
  sub.setAttribute('role', 'tablist');
  sub.setAttribute('aria-label', 'VJ sections');
  const main = document.createElement('div');
  main.className = 'ga-compact-main';
  main.setAttribute('role', 'tablist');
  nav.append(sub, main);

  const tabButtons = TABS.map((tab) =>
    tabButton(tab.id, `<svg viewBox="0 0 24 24" aria-hidden="true">${tab.icon}</svg><span>${tab.label}</span>`, () => select(tab.id)),
  );
  main.append(...tabButtons);
  const subButtons = VJ_SECTIONS.map((section) =>
    tabButton(section.id, section.label, () => selectVj(section.id)),
  );
  sub.append(...subButtons);

  let switchButton: HTMLButtonElement | null = null;
  if (options.onMobileLayout) {
    switchButton = document.createElement('button');
    switchButton.type = 'button';
    switchButton.className = 'ga-compact-switch';
    switchButton.textContent = 'Switch to mobile layout';
    switchButton.addEventListener('click', options.onMobileLayout);
  }

  let current: CompactTab = read(TAB_KEY, ['canvas', 'layers', 'inspector', 'tools'] as const, 'canvas');
  let vjSection: VjSection = read(VJ_KEY, ['clips', 'media', 'effects'] as const, 'clips');

  function paint(): void {
    root.dataset.compactTab = current;
    root.dataset.compactVj = vjSection;
    for (const button of tabButtons) button.setAttribute('aria-selected', String(button.dataset.id === current));
    for (const button of subButtons) button.setAttribute('aria-selected', String(button.dataset.id === vjSection));
    if (switchButton && current === 'tools') {
      const toolbar = document.querySelector('header.toolbar');
      if (toolbar && switchButton.parentElement !== toolbar) toolbar.appendChild(switchButton);
    }
  }

  function select(tab: CompactTab): void {
    if (tab === 'vj' && !vjOpen()) (document.querySelector('button.vj-btn') as HTMLButtonElement | null)?.click();
    current = tab;
    if (tab !== 'vj') store(TAB_KEY, tab);
    paint();
  }

  function selectVj(section: VjSection): void {
    vjSection = section;
    store(VJ_KEY, section);
    paint();
  }

  // Exit VJ inside the mixer unmounts it; show the canvas then.
  const observer = new MutationObserver(() => {
    if (current === 'vj' && !vjOpen()) {
      current = 'canvas';
      paint();
    }
  });

  function apply(): void {
    if (query.matches) {
      root.classList.add('ga-compact');
      if (!nav.isConnected) document.body.appendChild(nav);
      if (current === 'vj' && !vjOpen()) current = 'canvas';
      paint();
      observer.observe(document.body, { childList: true, subtree: true });
    } else {
      root.classList.remove('ga-compact');
      delete root.dataset.compactTab;
      delete root.dataset.compactVj;
      nav.remove();
      switchButton?.remove();
      observer.disconnect();
    }
    // Canvas, sidebars and their resize observers re-measure on resize.
    window.dispatchEvent(new Event('resize'));
  }

  query.addEventListener('change', apply);
  apply();
}
