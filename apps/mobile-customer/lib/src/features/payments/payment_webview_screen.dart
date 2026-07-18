import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:ui_kit/ui_kit.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:webview_flutter/webview_flutter.dart';

import '../../services/missing_providers.dart';

/// In-app WebView for Midtrans / Duitku.
///
/// Backend Midtrans returns a Snap **token** (no scheme). Callers must pass a
/// full URL (see [PaymentService.resolveMidtransPaymentUrl]).
/// On finish (success/fail/close) → invoice detail, not payment method picker.
class PaymentWebViewScreen extends ConsumerStatefulWidget {
  const PaymentWebViewScreen({
    required this.paymentUrl,
    required this.invoiceId,
    super.key,
  });

  final String paymentUrl;
  final String invoiceId;

  @override
  ConsumerState<PaymentWebViewScreen> createState() =>
      _PaymentWebViewScreenState();
}

class _PaymentWebViewScreenState extends ConsumerState<PaymentWebViewScreen> {
  late final WebViewController _controller;
  bool _isLoading = true;
  bool _handledCompletion = false;
  String? _loadError;

  late final IspThemeColors isp;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    isp = context.isp;
  }

  @override
  void initState() {
    super.initState();
    final uri = _safeUri(widget.paymentUrl);
    if (uri == null) {
      _loadError =
          'URL pembayaran tidak valid (missing scheme). Value: ${widget.paymentUrl}';
      _controller = WebViewController();
      return;
    }

    _controller = WebViewController()
      ..setJavaScriptMode(JavaScriptMode.unrestricted)
      ..setNavigationDelegate(
        NavigationDelegate(
          onPageStarted: (_) {
            if (mounted) {
              setState(() {
                _isLoading = true;
                _loadError = null;
              });
            }
          },
          onPageFinished: (_) {
            if (mounted) setState(() => _isLoading = false);
          },
          onWebResourceError: (error) {
            // Ignore subframe / cancelled noise; surface main-frame failures.
            if (error.isForMainFrame != true) return;
            if (mounted) {
              setState(() {
                _isLoading = false;
                _loadError = error.description.isNotEmpty
                    ? error.description
                    : 'Gagal memuat halaman pembayaran';
              });
            }
          },
          onNavigationRequest: _onNavigationRequest,
        ),
      )
      ..loadRequest(uri);
  }

  /// Parse only http(s). Bare Snap tokens must be converted before this screen.
  Uri? _safeUri(String raw) {
    final s = raw.trim();
    if (s.isEmpty) return null;
    final uri = Uri.tryParse(s);
    if (uri == null || !uri.hasScheme) return null;
    final scheme = uri.scheme.toLowerCase();
    if (scheme != 'http' && scheme != 'https') return null;
    return uri;
  }

  NavigationDecision _onNavigationRequest(NavigationRequest request) {
    final url = request.url;
    final lower = url.toLowerCase();
    final uri = Uri.tryParse(url);

    // Non-http schemes (gojek://, shopeeid://, intent://, tel:, etc.) → external.
    if (uri != null &&
        uri.hasScheme &&
        uri.scheme != 'http' &&
        uri.scheme != 'https') {
      _openExternal(uri);
      return NavigationDecision.prevent;
    }

    if (!_handledCompletion && _isResultUrl(lower)) {
      _handledCompletion = true;
      final isSuccess = _isSuccessUrl(lower);
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (!mounted) return;
        if (isSuccess) {
          _showResultAndPop(
            title: 'Pembayaran Berhasil',
            message: 'Terima kasih, pembayaran Anda telah diproses.',
            icon: Icons.check_circle,
            color: isp.success,
          );
        } else {
          _showResultAndPop(
            title: 'Pembayaran Gagal',
            message: 'Pembayaran tidak berhasil. Silakan coba lagi.',
            icon: Icons.error,
            color: isp.danger,
          );
        }
      });
    }
    return NavigationDecision.navigate;
  }

  Future<void> _openExternal(Uri uri) async {
    try {
      await launchUrl(uri, mode: LaunchMode.externalApplication);
    } catch (_) {
      // App not installed — stay on page; user can pick another method.
    }
  }

  bool _isResultUrl(String url) {
    return url.contains('/payment/result') ||
        url.contains('/payment/finish') ||
        url.contains('/payment/unfinish') ||
        url.contains('/payment/error') ||
        url.contains('/payment/callback') ||
        url.contains('transaction_status=') ||
        url.contains('/finish') ||
        url.contains('/unfinish') ||
        url.contains('/error') ||
        url.contains('status=success') ||
        url.contains('status=failed') ||
        url.contains('result=') ||
        (url.contains('/midtrans') && url.contains('status')) ||
        url.contains('/pay/');
  }

  bool _isSuccessUrl(String url) {
    if (url.contains('transaction_status=settlement') ||
        url.contains('transaction_status=capture')) {
      return true;
    }
    if (url.contains('status=success') || url.contains('/finish')) {
      return true;
    }
    if (url.contains('status=error') ||
        url.contains('status=failed') ||
        url.contains('unfinish') ||
        url.contains('transaction_status=deny') ||
        url.contains('transaction_status=cancel') ||
        url.contains('transaction_status=expire')) {
      return false;
    }
    // /pay/{id} finish callback without explicit fail → treat pending/success UI
    if (url.contains('/pay/') && !url.contains('status=error')) {
      return true;
    }
    return false;
  }

  void _exitToInvoice() {
    ref.invalidate(invoiceByIdProvider(widget.invoiceId));
    if (!mounted) return;
    // Skip payment method picker → invoice detail.
    context.go('/invoices/${widget.invoiceId}');
  }

  void _showResultAndPop({
    required String title,
    required String message,
    required IconData icon,
    required Color color,
  }) {
    showDialog(
      context: context,
      barrierDismissible: false,
      builder: (_) => AlertDialog(
        icon: Icon(icon, color: color, size: 48),
        title: Text(title),
        content: Text(message),
        actions: [
          TextButton(
            onPressed: () {
              Navigator.of(context).pop();
              _exitToInvoice();
            },
            child: const Text('OK'),
          ),
        ],
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Pembayaran'),
        leading: IconButton(
          icon: const Icon(Icons.close),
          onPressed: _exitToInvoice,
        ),
      ),
      body: _loadError != null
          ? Center(
              child: Padding(
                padding: const EdgeInsets.all(24),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(Icons.error_outline, size: 48, color: isp.danger),
                    const SizedBox(height: 12),
                    Text(
                      'Gagal memuat pembayaran',
                      style: TextStyle(
                        fontWeight: FontWeight.w600,
                        color: isp.textPrimary,
                      ),
                    ),
                    const SizedBox(height: 8),
                    Text(
                      _loadError!,
                      textAlign: TextAlign.center,
                      style: TextStyle(fontSize: 12, color: isp.textMuted),
                    ),
                    const SizedBox(height: 16),
                    FilledButton(
                      onPressed: _exitToInvoice,
                      child: const Text('Kembali'),
                    ),
                  ],
                ),
              ),
            )
          : Stack(
              children: [
                WebViewWidget(controller: _controller),
                if (_isLoading)
                  const Center(child: CircularProgressIndicator()),
              ],
            ),
    );
  }
}
