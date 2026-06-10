import 'package:equatable/equatable.dart';
import 'package:json_annotation/json_annotation.dart';

part 'user_model.g.dart';

/// User account — used across all 3 apps (customer, admin, superadmin).
@JsonSerializable()
class UserModel extends Equatable {
  const UserModel({
    required this.id,
    required this.email,
    required this.name,
    required this.role,
    required this.isSuperAdmin,
    this.avatarUrl,
    this.tenantId,
    this.tenantSlug,
    this.tenantRole,
    this.twoFactorEnabled = false,
    @JsonKey(name: 'enforce_2fa') this.enforce2fa = false,
    this.permissions = const [],
    this.phone,
  });

  factory UserModel.fromJson(Map<String, dynamic> json) => _$UserModelFromJson(json);
  Map<String, dynamic> toJson() => _$UserModelToJson(this);

  final String id;
  final String email;
  final String name;

  /// Role: `super_admin`, `tenant_admin`, `staff`, `technician`, `customer`.
  final String role;

  @JsonKey(name: 'is_super_admin')
  final bool isSuperAdmin;

  @JsonKey(name: 'avatar_url')
  final String? avatarUrl;

  @JsonKey(name: 'tenant_id')
  final String? tenantId;

  @JsonKey(name: 'tenant_slug')
  final String? tenantSlug;

  @JsonKey(name: 'tenant_role')
  final String? tenantRole;

  @JsonKey(name: 'two_factor_enabled')
  final bool twoFactorEnabled;

  @JsonKey(name: 'enforce_2fa')
  final bool enforce2fa;

  final String? phone;

  /// Permission keys for granular RBAC checks.
  final List<String> permissions;

  bool get isCustomer => role == 'customer';
  bool get isStaff => role == 'staff' || role == 'technician';
  bool get isTenantAdmin => role == 'tenant_admin';
  bool get isSuperAdminUser => isSuperAdmin || role == 'super_admin';

  bool can(String action, String resource) {
    return permissions.contains('$action:$resource') ||
        permissions.contains('manage:$resource') ||
        isSuperAdminUser;
  }

  @override
  List<Object?> get props => [
        id,
        email,
        name,
        role,
        isSuperAdmin,
        avatarUrl,
        tenantId,
        tenantSlug,
        tenantRole,
        twoFactorEnabled,
        enforce2fa,
        permissions,
      ];
}
