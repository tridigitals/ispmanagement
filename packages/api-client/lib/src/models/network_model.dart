import 'package:equatable/equatable.dart';
import 'package:json_annotation/json_annotation.dart';

part 'network_model.g.dart';

/// Snapshot of the customer's current network status.
@JsonSerializable()
class NetworkStatusModel extends Equatable {
  const NetworkStatusModel({
    required this.subscriptionId,
    required this.isOnline,
    required this.lastSeen,
    this.downloadMbps,
    this.uploadMbps,
    this.latencyMs,
    this.ipAddress,
    this.macAddress,
    this.routerName,
  });

  factory NetworkStatusModel.fromJson(Map<String, dynamic> json) =>
      _$NetworkStatusModelFromJson(json);
  Map<String, dynamic> toJson() => _$NetworkStatusModelToJson(this);

  @JsonKey(name: 'subscription_id')
  final String subscriptionId;
  @JsonKey(name: 'is_online')
  final bool isOnline;
  @JsonKey(name: 'last_seen')
  final DateTime lastSeen;
  @JsonKey(name: 'download_mbps')
  final double? downloadMbps;
  @JsonKey(name: 'upload_mbps')
  final double? uploadMbps;
  @JsonKey(name: 'latency_ms')
  final int? latencyMs;
  @JsonKey(name: 'ip_address')
  final String? ipAddress;
  @JsonKey(name: 'mac_address')
  final String? macAddress;
  @JsonKey(name: 'router_name')
  final String? routerName;

  @override
  List<Object?> get props => [
        subscriptionId,
        isOnline,
        lastSeen,
        downloadMbps,
        uploadMbps,
        latencyMs,
        ipAddress,
      ];
}

/// Traffic usage for a billing period.
@JsonSerializable()
class TrafficUsageModel extends Equatable {
  const TrafficUsageModel({
    required this.subscriptionId,
    required this.periodStart,
    required this.periodEnd,
    required this.bytesUsed,
    required this.bytesQuota,
    this.bytesPeak,
  });

  factory TrafficUsageModel.fromJson(Map<String, dynamic> json) =>
      _$TrafficUsageModelFromJson(json);
  Map<String, dynamic> toJson() => _$TrafficUsageModelToJson(this);

  @JsonKey(name: 'subscription_id')
  final String subscriptionId;
  @JsonKey(name: 'period_start')
  final DateTime periodStart;
  @JsonKey(name: 'period_end')
  final DateTime periodEnd;
  @JsonKey(name: 'bytes_used')
  final int bytesUsed;
  @JsonKey(name: 'bytes_quota')
  final int bytesQuota;
  @JsonKey(name: 'bytes_peak')
  final int? bytesPeak;

  double get usagePercent => bytesQuota == 0 ? 0 : (bytesUsed / bytesQuota).clamp(0, 1);

  @override
  List<Object?> get props => [subscriptionId, periodStart, periodEnd, bytesUsed, bytesQuota];
}
