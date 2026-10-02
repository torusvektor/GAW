/// <reference types="vitest/config" />
// Hosted browser build: a static site that runs in Safari, Chrome and
// Firefox on macOS, iOS and Android, installable as a PWA.
//
//   npm run build:web          → dist-web/ (relative paths, any host/subfolder)
//   npm run preview:web        → serve dist-web/ locally on :4173
//
// Reuses the desktop Vite config and only swaps the entry script, adds a
// web manifest and an offline service worker.
import { defineConfig, mergeConfig, type Plugin } from 'vite';
import { readFileSync, readdirSync, existsSync } from 'fs';
import { resolve } from 'path';
import baseConfig from './vite.config';

const pkg = JSON.parse(readFileSync('./package.json', 'utf-8'));

function webAppPlugin(): Plugin {
  const iconsDir = resolve(__dirname, 'icons');
  return {
    name: 'ghost-arcade-web',
    transformIndexHtml: {
      order: 'pre',
      handler(html) {
        return html
          .replace('/src/main.ts', '/src/web-main.ts')
          .replace('./manifest.json', './manifest.webmanifest')
          // The legacy companion SW registration is replaced by web-main.ts.
          .replace(/<script>\s*\/\/ Register service worker[\s\S]*?<\/script>/, '');
      },
    },
    generateBundle(_opts, bundle) {
      const icons = existsSync(iconsDir)
        ? readdirSync(iconsDir).filter((f) => /^icon-\d+\.webp$/.test(f))
        : [];
      for (const file of icons) {
        this.emitFile({ type: 'asset', fileName: `icons/${file}`, source: readFileSync(resolve(iconsDir, file)) });
      }
      const manifest = {
        name: 'Ghost Arcade',
        short_name: 'Ghost Arcade',
        description: 'Projection mapping & VJ software in the browser',
        id: './',
        start_url: './',
        scope: './',
        display: 'standalone',
        orientation: 'any',
        background_color: '#0a0a0c',
        theme_color: '#0a0a0c',
        categories: ['entertainment', 'music'],
        icons: icons.map((file) => {
          const size = file.match(/\d+/)![0];
          return { src: `icons/${file}`, type: 'image/webp', sizes: `${size}x${size}`, purpose: 'any' };
        }),
      };
      this.emitFile({ type: 'asset', fileName: 'manifest.webmanifest', source: JSON.stringify(manifest, null, 2) });

      // App shell precached at install time: HTML plus the entry chunk and
      // its static imports. Lazy chunks (editor, presets, 3D…) and public
      // assets are cached on first use by the service worker instead, so a
      // phone doesn't download the whole desktop editor up front.
      const shell = new Set(['./', 'index.html', 'manifest.webmanifest', 'logo.png', 'favicon.ico']);
      const visit = (name: string) => {
        const chunk = bundle[name];
        if (!chunk || shell.has(name)) return;
        shell.add(name);
        if (chunk.type === 'chunk') {
          chunk.imports.forEach(visit);
          chunk.viteMetadata?.importedCss.forEach((css) => shell.add(css));
        }
      };
      for (const [name, chunk] of Object.entries(bundle)) {
        if (chunk.type === 'chunk' && chunk.isEntry && chunk.facadeModuleId?.endsWith('index.html')) visit(name);
      }
      const sw = readFileSync(resolve(__dirname, 'web/sw.template.js'), 'utf-8')
        .replace('__CACHE_VERSION__', `${pkg.version}-${Date.now().toString(36)}`)
        .replace('__SHELL__', JSON.stringify([...shell]));
      this.emitFile({ type: 'asset', fileName: 'web-sw.js', source: sw });
    },
  };
}

export default mergeConfig(
  baseConfig,
  defineConfig({
    plugins: [webAppPlugin()],
    base: './',
    build: {
      outDir: 'dist-web',
      emptyOutDir: true,
    },
    preview: {
      port: 4173,
      host: true,
    },
  }),
);
