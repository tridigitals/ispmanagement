import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:web_socket_channel/web_socket_channel.dart';

import '../api/api_endpoints.dart';
import '../auth/auth_token_storage.dart';

/// Lightweight WebSocket client for realtime updates
/// (notifications, ticket replies, payment status).
///
/// Features:
/// - Auto-reconnect with exponential backoff (2s → 30s cap)
/// - Heartbeat ping every 25s to detect dead connections
/// - `forceReconnect()` for app-resume scenarios
/// - Broadcast stream so multiple widgets can listen simultaneously
class RealtimeClient {
  RealtimeClient({
    required this.baseUrl,
    required this.tokenStorage,
  });

  /// ws:// or wss:// root (e.g. `wss://api-isp-management.tridigitals.com`).
  final String baseUrl;
  final AuthTokenStorage tokenStorage;

  WebSocketChannel? _channel;
  StreamController<Map<String, dynamic>>? _controller;
  Timer? _reconnectTimer;
  Timer? _heartbeatTimer;
  Duration _reconnectDelay = const Duration(seconds: 2);
  bool _disposed = false;
  bool _connecting = false;

  /// Whether the WebSocket is currently connected.
  bool get isConnected => _channel != null;

  Stream<Map<String, dynamic>> get stream {
    _controller ??= StreamController<Map<String, dynamic>>.broadcast(
      onListen: connect,
      onCancel: () {
        // keep socket open — broadcast stream may have multiple listeners
      },
    );
    return _controller!.stream;
  }

  Future<void> connect() async {
    if (_disposed) return;
    if (_connecting) return; // prevent concurrent connect attempts
    if (_channel != null) return; // already connected

    _connecting = true;
    try {
      final token = await tokenStorage.readToken();
      if (token == null) {
        // Token not yet available (pre-login). Schedule retry.
        _connecting = false;
        _scheduleReconnect();
        return;
      }

      final wsUrl = baseUrl
          .replaceFirst('https://', 'wss://')
          .replaceFirst('http://', 'ws://');
      final uri = Uri.parse('$wsUrl${ApiEndpoints.wsRealtime}?token=$token');

      _channel = WebSocketChannel.connect(uri);

      // Wait for connection to be ready
      await _channel!.ready.timeout(
        const Duration(seconds: 10),
        onTimeout: () {
          debugPrint('[WS] Connection timeout (10s)');
          throw TimeoutException('WebSocket connection timeout');
        },
      );

      _channel!.stream.listen(
        (raw) {
          try {
            final data = json.decode(raw as String) as Map<String, dynamic>;
            _controller?.add(data);
          } catch (_) {
            // ignore malformed payloads
          }
        },
        onError: (e) {
          debugPrint('[WS] Stream error: $e');
          _stopHeartbeat();
          _scheduleReconnect();
        },
        onDone: () {
          debugPrint('[WS] Stream closed');
          _stopHeartbeat();
          _scheduleReconnect();
        },
        cancelOnError: true,
      );

      _reconnectDelay = const Duration(seconds: 2);
      _startHeartbeat();
      debugPrint('[WS] Connected to $uri');
    } catch (e) {
      debugPrint('[WS] Connect failed: $e');
      _channel?.sink.close();
      _channel = null;
      _scheduleReconnect();
    } finally {
      _connecting = false;
    }
  }

  /// Force close and reconnect. Call this when app resumes from background
  /// or when the connection may be silently dead.
  void forceReconnect() {
    debugPrint('[WS] Force reconnect requested');
    _channel?.sink.close();
    _channel = null;
    _stopHeartbeat();
    _reconnectTimer?.cancel();
    _reconnectDelay = const Duration(seconds: 1); // fast reconnect
    connect();
  }

  void _startHeartbeat() {
    _stopHeartbeat();
    _heartbeatTimer = Timer.periodic(
      const Duration(seconds: 25),
      (_) {
        // Send a ping frame. If the connection is dead, the send will fail
        // and trigger onError → reconnect.
        try {
          _channel?.sink.add(json.encode({'type': 'ping'}));
        } catch (e) {
          debugPrint('[WS] Heartbeat send failed: $e');
          forceReconnect();
        }
      },
    );
  }

  void _stopHeartbeat() {
    _heartbeatTimer?.cancel();
    _heartbeatTimer = null;
  }

  void _scheduleReconnect() {
    _channel?.sink.close();
    _channel = null;
    if (_disposed) return;
    _reconnectTimer?.cancel();
    debugPrint('[WS] Reconnecting in ${_reconnectDelay.inSeconds}s...');
    _reconnectTimer = Timer(_reconnectDelay, connect);
    // exponential backoff capped at 30s
    final next = _reconnectDelay * 2;
    _reconnectDelay =
        next > const Duration(seconds: 30) ? const Duration(seconds: 30) : next;
  }

  Future<void> dispose() async {
    _disposed = true;
    _reconnectTimer?.cancel();
    _stopHeartbeat();
    await _channel?.sink.close();
    await _controller?.close();
  }
}
