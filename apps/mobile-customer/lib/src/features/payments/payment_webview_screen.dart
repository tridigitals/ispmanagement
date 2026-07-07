import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import 'package:ui_kit/ui_kit.dart';
import 'package:webview_flutter/webview_flutter.dart';

/// In-app WebView screen for payment gateways (Midtrans / Duitku).
/// Loads the [paymentUrl] and monitors navigation for success/failure redirects.
class PaymentWebViewScreen extends StatefulWidget {
  const PaymentWebViewScreen({
    required this.paymentUrl,
    required this.invoiceId,
    super.key,
  });

  final String paymentUrl;
  final String invoiceId;

  @override
  State<PaymentWebViewScreen> createState() => _PaymentWebViewScreenState();
}

class _PaymentWebViewScreenState extends State<PaymentWebViewScreen> {
  late final WebViewController _controller;
  bool _isLoading = true;
  bool _handledCompletion = false;

  late final IspThemeColors isp;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    isp = context.isp;
  }

  @override
  void initState() {
    super.initState();
    _controller = WebViewController()
      ..setJavaScriptMode(JavaScriptMode.unrestricted)
      ..setNavigationDelegate(
        NavigationDelegate(
          onPageStarted: (_) {
            if (mounted) setState(() => _isLoading = true);
          },
          onPageFinished: (_) {
            if (mounted) setState(() => _isLoading = false);
          },
          onNavigationRequest: _onNavigationRequest,
        ),
      )
      ..loadRequest(Uri.parse(widget.paymentUrl));
  }

  NavigationDecision _onNavigationRequest(NavigationRequest request) {
    final url = request.url.toLowerCase();

    // Detect payment result redirects from Midtrans / Duitku / common patterns
    if (!_handledCompletion && _isResultUrl(url)) {
      _handledCompletion = true;
      final isSuccess = _isSuccessUrl(url);
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

  /// Returns true if the URL looks like a payment result/callback page.
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
        url.contains('/midtrans') && url.contains('status');
  }

  /// Returns true if the URL indicates a successful payment.
  bool _isSuccessUrl(String url) {
    // Midtrans-style
    if (url.contains('transaction_status=settlement') ||
        url.contains('transaction_status=capture')) {
      return true;
    }
    // Duitku-style or generic
    if (url.contains('status=success') || url.contains('/finish')) {
      return true;
    }
    // Default to success if it's a result URL but not explicitly error/unfinish
    if (!url.contains('error') &&
        !url.contains('unfinish') &&
        !url.contains('status=failed')) {
      return true;
    }
    return false;
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
              Navigator.of(context).pop(); // dismiss dialog
              // Pop back to invoices list or home
              context.pop();
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
          onPressed: () => context.pop(),
        ),
      ),
      body: Stack(
        children: [
          WebViewWidget(controller: _controller),
          if (_isLoading) const Center(child: CircularProgressIndicator()),
        ],
      ),
    );
  }
}
