import type { Handle } from '@sveltejs/kit';

/** Security headers applied to all HTML responses. */
export const handle: Handle = async ({ event, resolve }) => {
  const response = await resolve(event);
  response.headers.set('X-Frame-Options', 'DENY');
  response.headers.set('X-Content-Type-Options', 'nosniff');
  response.headers.set('Referrer-Policy', 'strict-origin-when-cross-origin');
  response.headers.set('Permissions-Policy', 'camera=(), microphone=(), geolocation=self');
  if (!response.headers.has('Content-Security-Policy')) {
    // Midtrans Snap needs script+frame; Duitku is full-page redirect (form-action).
    // connect/img stay broad for MapLibre tiles + payment XHR. Tighten later if needed.
    response.headers.set(
      'Content-Security-Policy',
      [
        "default-src 'self'",
        "script-src 'self' 'unsafe-inline' https://app.midtrans.com https://app.sandbox.midtrans.com",
        "frame-src 'self' https://*.midtrans.com https://app.midtrans.com https://app.sandbox.midtrans.com",
        "connect-src 'self' https:",
        "img-src 'self' data: blob: https:",
        "style-src 'self' 'unsafe-inline'",
        "font-src 'self' data:",
        "worker-src 'self' blob:",
        "child-src 'self' blob:",
        "object-src 'none'",
        "base-uri 'self'",
        "form-action 'self' https://sandbox.duitku.com https://passport.duitku.com",
        "frame-ancestors 'none'",
      ].join('; ')
    );
  }
  return response;
};
