// Service worker for the hosted web build. Generated into dist-web/web-sw.js
// by vite.config.web.ts — the cache version and shell list are filled in at
// build time.
//
// - App shell (HTML + hashed JS/CSS) is precached so the app opens offline.
// - Navigations are network-first so a new deploy is picked up on reload.
// - Other same-origin GETs (shaders, presets, wasm…) are cache-first and
//   stored on first use. Hashed files never change, so this is safe.

const CACHE = 'ga-web-__CACHE_VERSION__';
const SHELL = __SHELL__;

self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open(CACHE).then((cache) =>
      // One missing file must not abort the whole install.
      Promise.all(SHELL.map((url) => cache.add(url).catch(() => {}))),
    ),
  );
  self.skipWaiting();
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) => Promise.all(keys.filter((k) => k.startsWith('ga-web-') && k !== CACHE).map((k) => caches.delete(k))))
      .then(() => self.clients.claim()),
  );
});

self.addEventListener('fetch', (event) => {
  const req = event.request;
  if (req.method !== 'GET') return;
  const url = new URL(req.url);
  if (url.origin !== self.location.origin) return;
  // Media is usually streamed with Range requests; leave it to the network.
  if (req.headers.has('range')) return;

  if (req.mode === 'navigate') {
    event.respondWith(
      fetch(req)
        .then((res) => {
          if (res.ok) {
            const copy = res.clone();
            caches.open(CACHE).then((c) => c.put('index.html', copy));
          }
          return res;
        })
        .catch(() => caches.match('index.html', { ignoreSearch: true })),
    );
    return;
  }

  event.respondWith(
    caches.match(req).then(
      (hit) =>
        hit ||
        fetch(req).then((res) => {
          if (res.ok && res.type === 'basic') {
            const copy = res.clone();
            caches.open(CACHE).then((c) => c.put(req, copy));
          }
          return res;
        }),
    ),
  );
});
