self.addEventListener('install', function(e) {
  self.skipWaiting();
});

self.addEventListener('activate', function(e) {
  self.registration.unregister();
});

self.addEventListener('fetch', function(event) {
  // Pass through everything
});
