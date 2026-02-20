/**
 * WebSocket Store for Real-time Sync
 *
 * Connects to backend WebSocket and listens for events.
 * When permission-related events are received, triggers checkAuth to refresh user data.
 */
import { writable, get } from 'svelte/store';
import { checkAuth, authVersion, token, isSuperAdmin, user } from './auth';
import { goto } from '$app/navigation';
import { browser } from '$app/environment';
import {
  handleNotificationReceived,
  handleUnreadCountUpdated,
  markAsRead,
  markAllAsRead,
  loadNotifications,
} from './notifications';
import { getApiBaseUrl } from '$lib/utils/apiUrl';

// WebSocket connection state
export const wsConnected = writable(false);
export const wsError = writable<string | null>(null);

// WebSocket event types (must match backend)
type WsEvent =
  | { type: 'role_created'; role_id: string }
  | { type: 'role_updated'; role_id: string }
  | { type: 'role_deleted'; role_id: string }
  | { type: 'member_updated'; user_id: string }
  | { type: 'permissions_changed' }
  | { type: 'maintenance_mode_changed'; enabled: boolean; message?: string }
  | { type: 'connected'; message: string }
  | { type: 'ping' }
  // Notification Events
  | {
      type: 'notification_received';
      user_id: string;
      tenant_id: string | null;
      id: string;
      title: string;
      message: string;
      notification_type: any;
      category: any;
      action_url: string | null;
      created_at: string;
    }
  | { type: 'notification_read'; id: string }
  | { type: 'notifications_cleared' }
  | { type: 'unread_count_updated'; user_id: string; count: number }
  | {
      type: 'support_ticket_message_created';
      user_id: string;
      tenant_id: string | null;
      ticket_id: string;
      message_id: string;
    };

let ws: WebSocket | null = null;
let reconnectTimeout: ReturnType<typeof setTimeout> | null = null;
let heartbeatInterval: ReturnType<typeof setInterval> | null = null;
let watchdogInterval: ReturnType<typeof setInterval> | null = null;
let reconnectAttempts = 0;
const RECONNECT_BASE_DELAY = 1200; // ms
const RECONNECT_MAX_DELAY = 30_000; // ms
const DEV = import.meta.env.DEV;
let manualClose = false;
let lastMessageAt = 0;
const HEARTBEAT_INTERVAL_MS = 25_000;
const CONNECT_TIMEOUT_MS = 10_000;
let connectStartedAt = 0;

// Auto-connect in browser when token becomes available (prevents "must reload" issues).
// Guarded with a global flag to avoid duplicate subscriptions across HMR.
const g = globalThis as any;
if (browser && !g.__ws_auto_init) {
  g.__ws_auto_init = true;
  token.subscribe((t) => {
    if (t) {
      connectWebSocket();
      startWatchdog();
    } else {
      disconnectWebSocket();
    }
  });
  window.addEventListener('online', () => connectWebSocket());
  document.addEventListener('visibilitychange', () => {
    if (!document.hidden) connectWebSocket();
  });
}

function parseJwtSub(jwt: string): string | null {
  // JWT: header.payload.signature (base64url)
  const parts = jwt.split('.');
  if (parts.length < 2) return null;
  const payload = parts[1];
  try {
    const b64 = payload.replace(/-/g, '+').replace(/_/g, '/');
    const pad = b64.length % 4 ? '='.repeat(4 - (b64.length % 4)) : '';
    const json = atob(b64 + pad);
    const data = JSON.parse(json);
    return typeof data?.sub === 'string' ? data.sub : null;
  } catch {
    return null;
  }
}

function currentUserId(): string | null {
  const u = get(user);
  if (u?.id) return u.id;
  const t = get(token);
  if (!t) return null;
  return parseJwtSub(t);
}

/**
 * Connect to WebSocket server
 */
export function connectWebSocket() {
  if (!browser) return;

  // Avoid creating duplicate connections (e.g. reactive auth changes, HMR, route changes).
  if (ws && (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING)) {
    return;
  }

  const currentToken = get(token);
  if (!currentToken) {
    // Token might not be ready yet; auto-init subscriber will connect once available.
    return;
  }

  // Determine WebSocket URL based on API_BASE
  // Prefer configured API base (works in DEV when backend is remote),
  // otherwise default to local embedded server in Tauri dev.
  const apiBase = getApiBaseUrl();
  let wsUrl = apiBase;

  // Replace protocol
  if (wsUrl.startsWith('https://')) {
    wsUrl = wsUrl.replace('https://', 'wss://');
  } else if (wsUrl.startsWith('http://')) {
    wsUrl = wsUrl.replace('http://', 'ws://');
  }

  // Append /ws endpoint (WebSocket is at /api/ws)
  if (wsUrl.endsWith('/')) {
    wsUrl += 'ws';
  } else {
    wsUrl += '/ws';
  }

  wsError.set(null);

  try {
    manualClose = false;
    connectStartedAt = Date.now();
    ws = new WebSocket(wsUrl);

    ws.onopen = () => {
      wsConnected.set(true);
      wsError.set(null);
      reconnectAttempts = 0;
      lastMessageAt = Date.now();
      startHeartbeat();
      if (DEV) console.debug('[WS] connected', wsUrl);
    };

    ws.onmessage = (event) => {
      try {
        lastMessageAt = Date.now();
        const data: WsEvent = JSON.parse(event.data);
        handleWsEvent(data);
      } catch (e) {
        console.warn('[WS] Failed to parse message:', event.data);
      }
    };

    ws.onerror = (error) => {
      console.error('[WS] Error:', error);
      wsError.set('WebSocket connection error');
      // Some environments can get "stuck" in CONNECTING without reliably firing onclose.
      // Watchdog will handle it, but scheduling here makes reconnect more aggressive.
      scheduleReconnect();
    };

    ws.onclose = (event) => {
      wsConnected.set(false);
      ws = null;
      stopHeartbeat();

      // Auto-reconnect unless explicitly closed by app.
      // Note: some servers close with 1000 on restarts; we still want to reconnect.
      if (!manualClose) {
        if (DEV) console.debug('[WS] closed', { code: event.code, reason: event.reason });
        scheduleReconnect();
      }
    };
  } catch (e) {
    console.error('[WS] Failed to create WebSocket:', e);
    wsError.set('Failed to connect to WebSocket');
    scheduleReconnect();
  }
}

function startWatchdog() {
  if (!browser) return;
  if (watchdogInterval) return;

  // Keep trying to connect even if initial connect never opens (common in browser when backend
  // starts late or the network is flaky). This prevents "must reload" cases.
  watchdogInterval = setInterval(() => {
    const t = get(token);
    if (!t) return;

    if (!ws) {
      connectWebSocket();
      return;
    }

    if (ws.readyState === WebSocket.CONNECTING) {
      const age = Date.now() - (connectStartedAt || 0);
      if (age > CONNECT_TIMEOUT_MS) {
        if (DEV) console.debug('[WS] connect timeout, forcing reconnect', { age });
        try {
          ws.close();
        } catch {
          // ignore
        }
      }
      return;
    }

    if (ws.readyState === WebSocket.CLOSED) {
      connectWebSocket();
      return;
    }
  }, 2500);
}

function startHeartbeat() {
  stopHeartbeat();

  if (!browser) return;

  heartbeatInterval = setInterval(() => {
    const t = get(token);
    if (!t) return;

    // If WS is gone, keep trying in the background.
    if (!ws || ws.readyState === WebSocket.CLOSED) {
      connectWebSocket();
      return;
    }

    if (ws.readyState !== WebSocket.OPEN) return;

    // Send a lightweight keepalive message (text). Server may ignore it; still keeps the pipe warm.
    try {
      ws.send('ping');
    } catch {
      try {
        ws.close();
      } catch {
        // ignore
      }
    }

    // If connection gets "stuck" (rare in dev), force a reconnect.
    const idleFor = Date.now() - (lastMessageAt || 0);
    if (idleFor > HEARTBEAT_INTERVAL_MS * 6) {
      if (DEV) console.debug('[WS] idle too long, reconnecting', { idleFor });
      try {
        ws.close();
      } catch {
        // ignore
      }
    }
  }, HEARTBEAT_INTERVAL_MS);
}

function stopHeartbeat() {
  if (heartbeatInterval) {
    clearInterval(heartbeatInterval);
    heartbeatInterval = null;
  }
}

function stopWatchdog() {
  if (watchdogInterval) {
    clearInterval(watchdogInterval);
    watchdogInterval = null;
  }
}

/**
 * Handle incoming WebSocket events
 */
async function handleWsEvent(event: WsEvent) {
  switch (event.type) {
    case 'connected':
      // Connection acknowledged
      break;

    case 'role_created':
    case 'role_updated':
    case 'role_deleted':
    case 'member_updated':
    case 'permissions_changed':
      // Permissions may have changed - refresh user data
      await checkAuth();
      // Also increment authVersion to force UI re-render
      authVersion.update((v) => v + 1);
      break;

    case 'maintenance_mode_changed':
      // Maintenance mode status changed - redirect non-superadmins to maintenance page
      console.log('[WS] Maintenance mode changed:', event.enabled);
      if (event.enabled && !get(isSuperAdmin)) {
        console.log('[WS] Redirecting to maintenance page...');
        goto('/maintenance');
      } else if (!event.enabled) {
        // If maintenance was disabled and user is on maintenance page, redirect to dashboard
        if (typeof window !== 'undefined' && window.location.pathname === '/maintenance') {
          goto('/dashboard');
        }
      }
      break;

    case 'ping':
      // Keep-alive ping, ignore
      break;

    // Notification Events
    case 'notification_received':
      // Ignore broadcasts meant for other users (backend WS is fan-out).
      {
        const uid = currentUserId();
        if (!uid) {
          if (DEV) console.debug('[WS] notification_received ignored (no user id yet)', event);
          return;
        }
        if (event.user_id !== uid) {
          if (DEV) console.debug('[WS] notification_received ignored (other user)', { uid, event });
          return;
        }
      }
      // @ts-ignore
      handleNotificationReceived({
        id: event.id,
        title: event.title,
        message: event.message,
        notification_type: event.notification_type,
        category: event.category,
        action_url: event.action_url,
        created_at: event.created_at,
        user_id: event.user_id,
        tenant_id: event.tenant_id,
        is_read: false,
      });
      break;

    case 'notification_read':
      markAsRead(event.id);
      break;

    case 'notifications_cleared':
      loadNotifications(1); // Reload to sync state
      break;

    case 'unread_count_updated':
      {
        const uid = currentUserId();
        if (!uid) {
          if (DEV) console.debug('[WS] unread_count_updated ignored (no user id yet)', event);
          return;
        }
        if (event.user_id !== uid) {
          if (DEV) console.debug('[WS] unread_count_updated ignored (other user)', { uid, event });
          return;
        }
      }
      handleUnreadCountUpdated(event.count);
      break;

    case 'support_ticket_message_created':
      {
        const uid = currentUserId();
        if (!uid) return;
        if (event.user_id !== uid) return;

        try {
          window.dispatchEvent(
            new CustomEvent('support_ticket_message', {
              detail: { ticket_id: event.ticket_id, message_id: event.message_id },
            }),
          );
        } catch {
          // ignore
        }
      }
      break;

    default:
      // Unknown event type
      break;
  }
}

/**
 * Schedule a reconnection attempt
 */
function scheduleReconnect() {
  if (reconnectTimeout) {
    clearTimeout(reconnectTimeout);
  }

  reconnectAttempts++;
  const exp = Math.min(10, reconnectAttempts);
  const jitter = Math.floor(Math.random() * 600);
  const delay = Math.min(RECONNECT_MAX_DELAY, RECONNECT_BASE_DELAY * 2 ** exp) + jitter;

  reconnectTimeout = setTimeout(() => {
    connectWebSocket();
  }, delay);
}

/**
 * Disconnect WebSocket
 */
export function disconnectWebSocket() {
  if (reconnectTimeout) {
    clearTimeout(reconnectTimeout);
    reconnectTimeout = null;
  }

  stopHeartbeat();
  stopWatchdog();

  if (ws) {
    manualClose = true;
    ws.close(1000, 'User logout');
    ws = null;
  }

  wsConnected.set(false);
  reconnectAttempts = 0;
}

/**
 * Send a message through WebSocket (if needed)
 */
export function sendWsMessage(message: string) {
  if (ws && ws.readyState === WebSocket.OPEN) {
    ws.send(message);
  }
}
