import 'package:equatable/equatable.dart';
import 'package:json_annotation/json_annotation.dart';

part 'work_order_model.g.dart';

enum WorkOrderStatus {
  @JsonValue('pending')
  pending,
  @JsonValue('assigned')
  assigned,
  @JsonValue('in_progress')
  inProgress,
  @JsonValue('completed')
  completed,
  @JsonValue('cancelled')
  cancelled,
}

/// Installation work order view (mirrors backend `InstallationWorkOrderView`).
@JsonSerializable()
class WorkOrderModel extends Equatable {
  const WorkOrderModel({
    required this.id,
    required this.tenantId,
    required this.subscriptionId,
    required this.customerId,
    required this.locationId,
    required this.status,
    required this.createdAt,
    required this.updatedAt,
    this.invoiceId,
    this.packageId,
    this.routerId,
    this.assignedTo,
    this.scheduledAt,
    this.completedAt,
    this.notes,
    this.customerName,
    this.customerPhone,
    this.locationLabel,
    this.locationLatitude,
    this.locationLongitude,
    this.packageName,
    this.packageProvisioningType,
    this.routerName,
    this.assignedToName,
    this.assignedToEmail,
    this.assignmentId,
    this.assignmentStatus,
    this.subscriptionStatus,
    this.subscriptionStartsAt,
    this.subscriptionGraceUntil,
    this.hasCustomerPackageInvoice = false,
    this.selectedZoneId,
    this.selectedZoneName,
    this.selectedNodeId,
    this.selectedNodeName,
    this.selectedNodeScore,
  });

  factory WorkOrderModel.fromJson(Map<String, dynamic> json) =>
      _$WorkOrderModelFromJson(json);
  Map<String, dynamic> toJson() => _$WorkOrderModelToJson(this);

  final String id;
  @JsonKey(name: 'tenant_id')
  final String tenantId;
  @JsonKey(name: 'subscription_id')
  final String subscriptionId;
  @JsonKey(name: 'invoice_id')
  final String? invoiceId;
  @JsonKey(name: 'customer_id')
  final String customerId;
  @JsonKey(name: 'location_id')
  final String locationId;
  @JsonKey(name: 'package_id')
  final String? packageId;
  @JsonKey(name: 'router_id')
  final String? routerId;
  @JsonKey(name: 'status')
  final String status;
  @JsonKey(name: 'assigned_to')
  final String? assignedTo;
  @JsonKey(name: 'scheduled_at')
  final DateTime? scheduledAt;
  @JsonKey(name: 'completed_at')
  final DateTime? completedAt;
  @JsonKey(name: 'notes')
  final String? notes;
  @JsonKey(name: 'created_at')
  final DateTime createdAt;
  @JsonKey(name: 'updated_at')
  final DateTime updatedAt;

  // Joined fields
  @JsonKey(name: 'customer_name')
  final String? customerName;
  @JsonKey(name: 'customer_phone')
  final String? customerPhone;
  @JsonKey(name: 'location_label')
  final String? locationLabel;
  @JsonKey(name: 'location_latitude')
  final double? locationLatitude;
  @JsonKey(name: 'location_longitude')
  final double? locationLongitude;
  @JsonKey(name: 'package_name')
  final String? packageName;
  @JsonKey(name: 'package_provisioning_type')
  final String? packageProvisioningType;
  @JsonKey(name: 'router_name')
  final String? routerName;
  @JsonKey(name: 'assigned_to_name')
  final String? assignedToName;
  @JsonKey(name: 'assigned_to_email')
  final String? assignedToEmail;
  @JsonKey(name: 'assignment_id')
  final String? assignmentId;
  @JsonKey(name: 'assignment_status')
  final String? assignmentStatus;
  @JsonKey(name: 'subscription_status')
  final String? subscriptionStatus;
  @JsonKey(name: 'subscription_starts_at')
  final DateTime? subscriptionStartsAt;
  @JsonKey(name: 'subscription_grace_until')
  final DateTime? subscriptionGraceUntil;
  @JsonKey(name: 'has_customer_package_invoice')
  final bool hasCustomerPackageInvoice;
  @JsonKey(name: 'selected_zone_id')
  final String? selectedZoneId;
  @JsonKey(name: 'selected_zone_name')
  final String? selectedZoneName;
  @JsonKey(name: 'selected_node_id')
  final String? selectedNodeId;
  @JsonKey(name: 'selected_node_name')
  final String? selectedNodeName;
  @JsonKey(name: 'selected_node_score')
  final double? selectedNodeScore;

  // Computed helpers
  WorkOrderStatus get statusEnum {
    switch (status) {
      case 'pending':
        return WorkOrderStatus.pending;
      case 'assigned':
        return WorkOrderStatus.assigned;
      case 'in_progress':
        return WorkOrderStatus.inProgress;
      case 'completed':
        return WorkOrderStatus.completed;
      case 'cancelled':
        return WorkOrderStatus.cancelled;
      default:
        return WorkOrderStatus.pending;
    }
  }

  bool get isPending => status == 'pending';
  bool get isAssigned => status == 'assigned';
  bool get isInProgress => status == 'in_progress';
  bool get isCompleted => status == 'completed';
  bool get isCancelled => status == 'cancelled';
  bool get isActive => !isCompleted && !isCancelled;

  String statusLabel() {
    switch (status) {
      case 'pending':
        return 'Menunggu';
      case 'assigned':
        return 'Ditugaskan';
      case 'in_progress':
        return 'Dikerjakan';
      case 'completed':
        return 'Selesai';
      case 'cancelled':
        return 'Dibatalkan';
      default:
        return status;
    }
  }

  String? get scheduledDateFormatted {
    if (scheduledAt == null) return null;
    return '${scheduledAt!.day}/${scheduledAt!.month}/${scheduledAt!.year}';
  }

  String? get scheduledTimeFormatted {
    if (scheduledAt == null) return null;
    return '${scheduledAt!.hour.toString().padLeft(2, '0')}:${scheduledAt!.minute.toString().padLeft(2, '0')}';
  }

  @override
  List<Object?> get props => [
        id,
        tenantId,
        subscriptionId,
        customerId,
        locationId,
        status,
        assignedTo,
        scheduledAt,
        completedAt,
        notes,
        createdAt,
        updatedAt,
        customerName,
        customerPhone,
        locationLabel,
        locationLatitude,
        locationLongitude,
        packageName,
        assignedToName,
      ];
}

/// Request to update work order status (complete/cancel).
@JsonSerializable()
class UpdateWorkOrderStatusRequest extends Equatable {
  const UpdateWorkOrderStatusRequest({
    this.notes,
    this.terminalAssetId,
    this.parentAssetId,
  });

  factory UpdateWorkOrderStatusRequest.fromJson(Map<String, dynamic> json) =>
      _$UpdateWorkOrderStatusRequestFromJson(json);
  Map<String, dynamic> toJson() => _$UpdateWorkOrderStatusRequestToJson(this);

  @JsonKey(name: 'notes')
  final String? notes;
  @JsonKey(name: 'terminal_asset_id')
  final String? terminalAssetId;
  @JsonKey(name: 'parent_asset_id')
  final String? parentAssetId;

  @override
  List<Object?> get props => [notes, terminalAssetId, parentAssetId];
}

/// Technician performance stats.
class TechnicianStats extends Equatable {
  const TechnicianStats({
    required this.totalAssigned,
    required this.completed,
    required this.inProgress,
    required this.pending,
    required this.cancelled,
    this.avgCompletionHours,
    this.completedToday,
    this.completedThisWeek,
  });

  final int totalAssigned;
  final int completed;
  final int inProgress;
  final int pending;
  final int cancelled;
  final double? avgCompletionHours;
  final int? completedToday;
  final int? completedThisWeek;

  double get completionRate =>
      totalAssigned > 0 ? (completed / totalAssigned * 100) : 0;

  @override
  List<Object?> get props => [
        totalAssigned,
        completed,
        inProgress,
        pending,
        cancelled,
        avgCompletionHours,
        completedToday,
        completedThisWeek,
      ];
}
