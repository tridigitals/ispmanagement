import 'dart:async';
import 'dart:convert';

import 'package:web_socket_channel/web_socket_channel.dart';

import 'api_endpoints.dart';
import 'auth_token_storage.dart';

/// Lightweight WebSocket client for realtime updates
/// (notifications, ticket replies, payment status).
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
  Duration _reconnectDelay = const Duration(seconds: 2);
  bool _disposed = false;

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
    final token = await tokenStorage.readToken();
    if (token == null) return;

    final wsUrl = baseUrl
        .replaceFirst('https://', 'wss://')
        .replaceFirst('http://', 'ws://');
    final uri = Uri.parse('$wsUrl${ApiEndpoints.wsRealtime}?token=$token');

    try {
      _channel = WebSocketChannel.connect(uri);
      _channel!.stream.listen(
        (raw) {
          try {
            final data = json.decode(raw as String) as Map<String, dynamic>;
            _controller?.add(data);
          } catch (_) {
            // ignore malformed payloads
          }
        },
        onError: (_) => _scheduleReconnect(),
        onDone: _scheduleReconnect,
        cancelOnError: true,
      );
      _reconnectDelay = const Duration(seconds: 2);
    } catch (_) {
      _scheduleReconnect();
    }
  }

  void _scheduleReconnect() {
    _channel?.sink.close();
    _channel = null;
    if (_disposed) return;
    _reconnectTimer?.cancel();
    _reconnectTimer = Timer(_reconnectDelay, connect);
    // exponential backoff capped at 30s
    final next = _reconnectDelay * 2;
    _reconnectDelay = next > const Duration(seconds: 30) ? const Duration(seconds: 30) : next;
  }

  Future<void> dispose() async {
    _disposed = true;
    _reconnectTimer?.cancel();
    await _channel?.sink.close();
    await _controller?.close();
  }
}
