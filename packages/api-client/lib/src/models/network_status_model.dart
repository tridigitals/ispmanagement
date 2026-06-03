import 'package:equatable/equatable.dart';
import 'package:json_annotation/json_annotation.dart';

part 'network_status_model.g.dart';

enum NetworkOperationalStatus {
  @JsonValue('operational')
  operational,
  @JsonValue('degraded')
  degraded,
  @JsonValue('partial_outage')
  partialOutage,
  @JsonValue('major_outage')
  majorOutage,
  @JsonValue('maintenance')
  maintenance,
}

extension on NetworkOperationalStatus {
  String get label {
    switch (this) {
      case NetworkOperationalStatus.operational:
        return 'Normal';
      case NetworkOperationalStatus.degraded:
        return 'Teranggu';
      case NetworkOperationalStatus.partialOutage:
        return 'Sebagian Tidak Aktif';
      case NetworkOperationalStatus.majorOutage:
        return 'Tidak Aktif';
      case NetworkOperationalStatus.maintenance:
        return 'Pemeliharaan';
    }
  }
}

@JsonSerializable()
class NetworkStatusModel extends Equatable {
  const NetworkStatusModel({
    required this.status,
    required this.area,
    this.message,
    this.affectedCustomers = 0,
    this.eta,
    this.lastUpdated,
  });

  factory NetworkStatusModel.fromJson(Map<String, dynamic> json) =>
      _$NetworkStatusModelFromJson(json);

  final NetworkOperationalStatus status;
  final String area;
  final String? message;
  final int affectedCustomers;
  final DateTime? eta;
  final DateTime? lastUpdated;

  String get statusLabel => status.label;

  /// True if customer experience is impacted.
  bool get isImpacted =>
      status == NetworkOperationalStatus.partialOutage ||
      status == NetworkOperationalStatus.majorOutage ||
      status == NetworkOperationalStatus.maintenance ||
      status == NetworkOperationalStatus.degraded;

  bool get isNormal => status == NetworkOperationalStatus.operational;

  Map<String, dynamic> toJson() => _$NetworkStatusModelToJson(this);

  @override
  List<Object?> get props => [status, area, message, affectedCustomers, eta];
}
