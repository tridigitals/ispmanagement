import 'package:json_annotation/json_annotation.dart';

part 'announcement_model.g.dart';

@JsonSerializable()
class AnnouncementModel {
  AnnouncementModel({
    required this.id,
    required this.title,
    required this.body,
    required this.severity,
    required this.mode,
    required this.format,
    this.tenantId,
    this.createdBy,
    this.coverFileId,
    this.audience = 'all',
    this.deliverInApp = true,
    this.deliverEmail = false,
    this.startsAt,
    this.endsAt,
    this.notifiedAt,
    this.createdAt,
    this.updatedAt,
  });

  final String id;
  @JsonKey(name: 'tenant_id')
  final String? tenantId;
  @JsonKey(name: 'created_by')
  final String? createdBy;
  @JsonKey(name: 'cover_file_id')
  final String? coverFileId;
  final String title;
  final String body;
  final String severity;
  final String audience;
  final String mode;
  final String format;
  @JsonKey(name: 'deliver_in_app')
  final bool deliverInApp;
  @JsonKey(name: 'deliver_email')
  final bool deliverEmail;
  @JsonKey(name: 'starts_at')
  final String? startsAt;
  @JsonKey(name: 'ends_at')
  final String? endsAt;
  @JsonKey(name: 'notified_at')
  final String? notifiedAt;
  @JsonKey(name: 'created_at')
  final String? createdAt;
  @JsonKey(name: 'updated_at')
  final String? updatedAt;

  factory AnnouncementModel.fromJson(Map<String, dynamic> json) =>
      _$AnnouncementModelFromJson(json);

  Map<String, dynamic> toJson() => _$AnnouncementModelToJson(this);

  /// Severity as display label
  String get severityLabel {
    switch (severity) {
      case 'info':
        return 'Info';
      case 'success':
        return 'Berhasil';
      case 'warning':
        return 'Peringatan';
      case 'error':
        return 'Error';
      default:
        return severity;
    }
  }

  /// Strip HTML tags for plain text preview
  String get plainBody {
    return body
        .replaceAll(RegExp(r'<[^>]*>'), '')
        .replaceAll('&nbsp;', ' ')
        .replaceAll('&amp;', '&')
        .replaceAll('&lt;', '<')
        .replaceAll('&gt;', '>')
        .trim();
  }
}
