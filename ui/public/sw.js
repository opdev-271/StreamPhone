// Minimal service worker: makes the page installable ("Add to Home screen").
// Everything is served live by the agent on the PC, so nothing is cached here.
self.addEventListener('install', () => self.skipWaiting());
self.addEventListener('activate', (e) => e.waitUntil(self.clients.claim()));
self.addEventListener('fetch', () => {});
