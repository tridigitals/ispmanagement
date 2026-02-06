//! WebSocket handler for real-time sync
//!
//! This module provides WebSocket support for broadcasting events to all connected clients.
//! When roles/permissions are updated, connected clients receive notifications to refresh their data.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{info, warn};

/// WebSocket event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsEvent {
    /// Role was created
    RoleCreated { role_id: String },
    /// Role was updated (permissions changed)
    RoleUpdated { role_id: String },
    /// Role was deleted
    RoleDeleted { role_id: String },
    /// Team member role changed
    MemberUpdated { user_id: String },
    /// Generic permissions refresh signal
    PermissionsChanged,
    /// Maintenance mode status changed
    MaintenanceModeChanged {
        enabled: bool,
        message: Option<String>,
    },
    /// Connection established acknowledgment
    Connected { message: String },
    /// Ping to keep connection alive
    Ping,
    /// New notification received
    NotificationReceived {
        user_id: String,
        tenant_id: Option<String>,
        id: String,
        title: String,
        message: String,
        notification_type: String,
        category: String,
        action_url: Option<String>,
        created_at: String,
    },
    /// Notification marked as read
    NotificationRead { id: String },
    /// All notifications marked as read
    NotificationsCleared,
    /// Unread count updated (optimization instead of sending full lists)
    UnreadCountUpdated { user_id: String, count: i64 },

    /// Support ticket message created (used to live-refresh ticket threads)
    SupportTicketMessageCreated {
        user_id: String,
        tenant_id: Option<String>,
        ticket_id: String,
        message_id: String,
    },
}

/// WebSocket connection manager
/// Uses broadcast channel to send events to all connected clients
#[derive(Clone)]
pub struct WsHub {
    /// Broadcast sender - clone this to send events
    tx: broadcast::Sender<WsEvent>,
}

impl WsHub {
    pub fn new() -> Self {
        // Create broadcast channel with capacity of 100 messages
        let (tx, _) = broadcast::channel(100);
        Self { tx }
    }

    /// Broadcast an event to all connected clients
    pub fn broadcast(&self, event: WsEvent) {
        let _ = self.tx.send(event);
    }

    /// Subscribe to events (for new connections)
    pub fn subscribe(&self) -> broadcast::Receiver<WsEvent> {
        self.tx.subscribe()
    }
}

impl Default for WsHub {
    fn default() -> Self {
        Self::new()
    }
}

/// WebSocket upgrade handler
pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<super::AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state.ws_hub.clone()))
}

/// Handle individual WebSocket connection
async fn handle_socket(socket: WebSocket, hub: Arc<WsHub>) {
    let (mut sender, mut receiver) = socket.split();

    // Subscribe to broadcast events
    let mut rx = hub.subscribe();

    // Send welcome message
    let welcome = WsEvent::Connected {
        message: "Connected to real-time sync".to_string(),
    };
    if let Ok(json) = serde_json::to_string(&welcome) {
        let _ = sender.send(Message::Text(json.into())).await;
    }

    info!("[WS] Client connected");

    // Spawn task to forward broadcast events to this client
    let mut send_task = tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&event) {
                if sender.send(Message::Text(json.into())).await.is_err() {
                    break; // Connection closed
                }
            }
        }
    });

    // Spawn task to handle incoming messages from client
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(_) => {
                    // Handle client messages if needed (e.g., ping/pong)
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    // Wait for either task to complete (connection closed)
    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }

    warn!("[WS] Client disconnected");
}
