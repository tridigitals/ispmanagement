import 'package:api_client/api_client.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'feature_providers.dart';

/// Poll a transaction's status (used while showing VA/QRIS instructions).
final paymentStatusProvider =
    FutureProvider.family<PaymentTransaction, String>((ref, txnId) async {
  final svc = ref.watch(paymentServiceProvider);
  final res = await svc.getTransaction(txnId);
  switch (res) {
    case Success(:final data):
      return data;
    case Failure(:final exception):
      throw Exception(exception.message);
  }
});

/// Fetch available payment channels for a given invoice.
final paymentChannelsProvider =
    FutureProvider.family<List<PaymentChannel>, String>((ref, invoiceId) async {
  final svc = ref.watch(paymentServiceProvider);
  final res = await svc.paymentChannels(invoiceId);
  switch (res) {
    case Success(:final data):
      return data;
    case Failure():
      return <PaymentChannel>[];
  }
});
