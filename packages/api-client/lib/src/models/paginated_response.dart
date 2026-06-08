/// Generic paginated API response used by list endpoints.
///
/// Backend returns: `{ "data": [...], "page": 1, "per_page": 20, "total": 123 }`
class PaginatedResponse<T> {
  const PaginatedResponse({
    required this.data,
    required this.page,
    required this.perPage,
    required this.total,
  });

  factory PaginatedResponse.fromJson(
    Map<String, dynamic> json,
    T Function(Map<String, dynamic>) parseItem,
  ) {
    final raw = (json['data'] as List<dynamic>?) ?? const [];
    return PaginatedResponse<T>(
      data: raw.map((e) => parseItem(e as Map<String, dynamic>)).toList(),
      page: (json['page'] as num?)?.toInt() ?? 1,
      perPage: (json['per_page'] as num?)?.toInt() ?? 20,
      total: (json['total'] as num?)?.toInt() ?? 0,
    );
  }

  final List<T> data;
  final int page;
  final int perPage;
  final int total;

  bool get hasMore => page * perPage < total;
}
