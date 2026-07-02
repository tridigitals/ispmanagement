import 'package:equatable/equatable.dart';
import 'package:json_annotation/json_annotation.dart';

part 'network_asset_model.g.dart';

/// Minimal network asset model for terminal asset selection.
@JsonSerializable()
class NetworkAssetListItemModel extends Equatable {
  const NetworkAssetListItemModel({
    required this.id,
    required this.assetType,
    required this.name,
    required this.status,
    this.code,
    this.serialNumber,
    this.customerId,
    this.locationId,
  });

  factory NetworkAssetListItemModel.fromJson(Map<String, dynamic> json) =>
      _$NetworkAssetListItemModelFromJson(json);
  Map<String, dynamic> toJson() => _$NetworkAssetListItemModelToJson(this);

  final String id;
  @JsonKey(name: 'asset_type')
  final String assetType;
  final String name;
  final String status;
  final String? code;
  @JsonKey(name: 'serial_number')
  final String? serialNumber;
  @JsonKey(name: 'customer_id')
  final String? customerId;
  @JsonKey(name: 'location_id')
  final String? locationId;

  String get displayLabel {
    final parts = <String>[name];
    if (code != null && code!.isNotEmpty) parts.add('($code)');
    if (serialNumber != null && serialNumber!.isNotEmpty) {
      parts.add('SN: $serialNumber');
    }
    return parts.join(' ');
  }

  @override
  List<Object?> get props => [id, assetType, name, status, code, serialNumber];
}
