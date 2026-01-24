/**
 * Service Worker for Push Notifications
 */

self.addEventListener('push', function (event) {
    if (event.data) {
        try {
            const data = event.data.json();

            const options = {
                body: data.message,
                icon: '/icon-192x192.png',
                badge: '/icon-96x96.png',
                tag: data.id, // Deduplicate notifications with same ID
                data: {
                    url: data.action_url || '/'
                }
            };

            event.waitUntil(
                clients.matchAll({ type: 'window', includeUncontrolled: true }).then(function (clientList) {
                    // Check if any client is focused/visible using the application
                    const isAppFocused = clientList.some(client =>
                        client.url.includes(self.location.origin) &&
                        client.visibilityState === 'visible' &&
                        ('focused' in client ? client.focused : true) // focused prop might not be available in all browsers
                    );

                    // If app is focused, we rely on the in-app WebSocket notification
                    if (isAppFocused) {
                        console.log('App is focused, suppressing push notification');
                        return;
                    }

                    return self.registration.showNotification(data.title, options);
                })
            );
        } catch (e) {
            console.error('Push event error:', e);
        }
    } else {
        console.log('Push event but no data');
    }
});

self.addEventListener('notificationclick', function (event) {
    event.notification.close();

    // Focus existing window or open new one
    event.waitUntil(
        clients.matchAll({ type: 'window', includeUncontrolled: true }).then(function (clientList) {
            // If action_url is present in data, use it
            const url = event.notification.data?.url || '/';

            // Check if there's already a tab open
            for (let i = 0; i < clientList.length; i++) {
                const client = clientList[i];
                // Simplify URL matching logic
                if (client.url.includes(self.location.origin) && 'focus' in client) {
                    // Optionally navigate to specific URL if needed, but focus is primary
                    if (url !== '/' && client.navigate) {
                        client.navigate(url);
                    }
                    return client.focus();
                }
            }
            if (clients.openWindow) {
                return clients.openWindow(url);
            }
        })
    );
});
