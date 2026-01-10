/**
 * WebSocket Store for Real-time Sync
 * 
 * Connects to backend WebSocket and listens for events.
 * When permission-related events are received, triggers checkAuth to refresh user data.
 */
import { writable, get } from 'svelte/store';
import { checkAuth, authVersion, token } from './auth';
import { browser } from '$app/environment';

// WebSocket connection state
export const wsConnected = writable(false);
export const wsError = writable<string | null>(null);

// WebSocket event types (must match backend)
type WsEvent =
    | { type: 'role_created', role_id: string }
    | { type: 'role_updated', role_id: string }
    | { type: 'role_deleted', role_id: string }
    | { type: 'member_updated', user_id: string }
    | { type: 'permissions_changed' }
    | { type: 'connected', message: string }
    | { type: 'ping' };

let ws: WebSocket | null = null;
let reconnectTimeout: ReturnType<typeof setTimeout> | null = null;
let reconnectAttempts = 0;
const MAX_RECONNECT_ATTEMPTS = 10;
const RECONNECT_DELAY = 3000; // 3 seconds

/**
 * Connect to WebSocket server
 */
export function connectWebSocket() {
    if (!browser) return;

    const currentToken = get(token);
    if (!currentToken) {
        return;
    }

    // Determine WebSocket URL based on API_BASE
    const apiBase = import.meta.env.VITE_API_URL || 'http://localhost:3000/api';
    let wsUrl = apiBase;

    // Replace protocol
    if (wsUrl.startsWith('https://')) {
        wsUrl = wsUrl.replace('https://', 'wss://');
    } else if (wsUrl.startsWith('http://')) {
        wsUrl = wsUrl.replace('http://', 'ws://');
    }

    // Append /ws endpoint
    if (wsUrl.endsWith('/')) {
        wsUrl += 'ws';
    } else {
        wsUrl += '/ws';
    }

    wsError.set(null);

    try {
        ws = new WebSocket(wsUrl);

        ws.onopen = () => {
            wsConnected.set(true);
            wsError.set(null);
            reconnectAttempts = 0;
        };

        ws.onmessage = (event) => {
            try {
                const data: WsEvent = JSON.parse(event.data);
                handleWsEvent(data);
            } catch (e) {
                console.warn('[WS] Failed to parse message:', event.data);
            }
        };

        ws.onerror = (error) => {
            console.error('[WS] Error:', error);
            wsError.set('WebSocket connection error');
        };

        ws.onclose = (event) => {
            wsConnected.set(false);
            ws = null;

            // Auto-reconnect if not intentionally closed
            if (event.code !== 1000 && reconnectAttempts < MAX_RECONNECT_ATTEMPTS) {
                scheduleReconnect();
            }
        };
    } catch (e) {
        console.error('[WS] Failed to create WebSocket:', e);
        wsError.set('Failed to connect to WebSocket');
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
            authVersion.update(v => v + 1);
            break;

        case 'ping':
            // Keep-alive ping, ignore
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
    const delay = RECONNECT_DELAY * reconnectAttempts; // Exponential backoff

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

    if (ws) {
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
