import 'package:api_client/api_client.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'service_providers.dart';
import 'package:result_dart/result_dart.dart';

/// List payment channels available for an invoice.
final paymentChannelsProvider =
    FutureProvider.family<List<PaymentChannel>, String>((ref, invoiceId) async {
  final svc = ref.watch(paymentServiceProvider);
  final res = await svc.getChannels(invoiceId);
  return switch (res) {
    Success(:final data) => data,
    Failure(:final exception) => throw exception.message,
  };
});

/// Create a new payment transaction (returns the txn to navigate to).
final createTransactionProvider = FutureProvider.family<
    Result<PaymentTransaction, ApiException>, _CreateTxnArgs>((ref, args) async {
  final svc = ref.watch(paymentServiceProvider);
  return svc.createTransaction(
    invoiceId: args.invoiceId,
    paymentChannelCode: args.channelCode,
  );
});

class _CreateTxnArgs {
  const _CreateTxnArgs(this.invoiceId, this.channelCode);
  final String invoiceId;
  final String channelCode;
  @override
  bool operator ==(Object other) =>
      other is _CreateTxnArgs &&
      other.invoiceId == invoiceId &&
      other.channelCode == channelCode;
  @override
  int get hashCode => Object.hash(invoiceId, channelCode);
}

/// Poll a transaction's status (used while showing VA/QRIS instructions).
final paymentStatusProvider =
    FutureProvider.family<PaymentTransaction, String>((ref, txnId) async {
  final svc = ref.watch(paymentServiceProvider);
  final res = await svc.getTransaction(txnId);
  return switch (res) {
    Success(:final data) => data,
    Failure(:final exception) => throw exception.message,
  };
});
