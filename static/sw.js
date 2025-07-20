// Service Worker for SSH AI Terminal
const CACHE_NAME = 'ssh-ai-terminal-v1';
const urlsToCache = [
    '/',
    '/index.html',
    '/css/reset.css',
    '/css/variables.css',
    '/css/components.css',
    '/css/layout.css',
    '/css/terminal.css',
    '/css/chat.css',
    '/css/responsive.css',
    '/js/app.js',
    '/js/modules/terminal.js',
    '/js/modules/ssh.js',
    '/js/modules/ai-chat.js',
    '/js/modules/config.js',
    '/js/modules/ui.js',
    '/js/modules/utils.js',
    '/manifest.json',
    '/favicon.ico'
];

// Install event - cache resources
self.addEventListener('install', event => {
    console.log('[ServiceWorker] Install');
    event.waitUntil(
        caches.open(CACHE_NAME)
            .then(cache => {
                console.log('[ServiceWorker] Caching app shell');
                return cache.addAll(urlsToCache);
            })
            .catch(error => {
                console.error('[ServiceWorker] Failed to cache:', error);
            })
    );
});

// Activate event - clean up old caches
self.addEventListener('activate', event => {
    console.log('[ServiceWorker] Activate');
    event.waitUntil(
        caches.keys().then(cacheNames => {
            return Promise.all(
                cacheNames.map(cacheName => {
                    if (cacheName !== CACHE_NAME) {
                        console.log('[ServiceWorker] Removing old cache:', cacheName);
                        return caches.delete(cacheName);
                    }
                })
            );
        })
    );
});

// Fetch event - serve from cache with network fallback
self.addEventListener('fetch', event => {
    // Skip non-GET requests
    if (event.request.method !== 'GET') {
        return;
    }

    // Skip WebSocket requests
    if (event.request.url.includes('/ws')) {
        return;
    }

    // Skip API requests - always fetch from network
    if (event.request.url.includes('/api/')) {
        return;
    }

    event.respondWith(
        caches.match(event.request)
            .then(response => {
                // Cache hit - return response
                if (response) {
                    return response;
                }

                // Clone the request
                const fetchRequest = event.request.clone();

                return fetch(fetchRequest)
                    .then(response => {
                        // Check if valid response
                        if (!response || response.status !== 200 || response.type !== 'basic') {
                            return response;
                        }

                        // Don't cache non-successful responses
                        if (response.status >= 300) {
                            return response;
                        }

                        // Clone the response
                        const responseToCache = response.clone();

                        caches.open(CACHE_NAME)
                            .then(cache => {
                                cache.put(event.request, responseToCache);
                            })
                            .catch(error => {
                                console.error('[ServiceWorker] Failed to cache response:', error);
                            });

                        return response;
                    })
                    .catch(error => {
                        console.error('[ServiceWorker] Fetch failed:', error);
                        
                        // Return offline page if available
                        return caches.match('/index.html');
                    });
            })
            .catch(error => {
                console.error('[ServiceWorker] Cache match failed:', error);
            })
    );
});

// Handle messages from the client
self.addEventListener('message', event => {
    if (event.data && event.data.type === 'SKIP_WAITING') {
        self.skipWaiting();
    }
});
