import 'package:api_client/api_client.dart';
import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:flutter_secure_storage/flutter_secure_storage.dart';

/// In-memory FlutterSecureStorage for unit tests.
class _InMemoryStorage implements FlutterSecureStorage {
  final Map<String, String> _data = {};
  @override
  Future<String?> read({required String key, IOSOptions? iOptions, AndroidOptions? aOptions, LinuxOptions? lOptions}) async => _data[key];
  @override
  Future<void> write({required String key, required String? value, IOSOptions? iOptions, AndroidOptions? aOptions, LinuxOptions? lOptions}) async {
    if (value == null) {
      _data.remove(key);
    } else {
      _data[key] = value;
    }
  }
  @override
  Future<void> delete({required String key, IOSOptions? iOptions, AndroidOptions? aOptions, LinuxOptions? lOptions}) async {
    _data.remove(key);
  }
  @override
  noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

void main() {
  group('AuthTokenStorage', () {
    late AuthTokenStorage storage;

    setUp(() {
      storage = AuthTokenStorage(storage: _InMemoryStorage());
    });

    test('save() persists all fields', () async {
      await storage.save(
        token: 'token-123',
        refreshToken: 'refresh-456',
        userId: 'u-1',
        tenantId: 't-1',
      );
      expect(await storage.readToken(), 'token-123');
      expect(await storage.readRefresh(), 'refresh-456');
      expect(await storage.readUserId(), 'u-1');
      expect(await storage.readTenantId(), 't-1');
    });

    test('clear() removes all', () async {
      await storage.save(token: 'tok');
      await storage.clear();
      expect(await storage.readToken(), null);
    });

    test('isExpired returns true past expiry', () async {
      await storage.save(token: 'tok', expiresAt: DateTime(2020));
      expect(await storage.isExpired(), true);
    });
  });

  group('ApiException.fromDio', () {
    test('extracts message from response data', () {
      final e = ApiException.fromDio(DioException(
        requestOptions: RequestOptions(path: '/x'),
        response: Response(
          requestOptions: RequestOptions(path: '/x'),
          statusCode: 400,
          data: {'message': 'Bad email'},
        ),
        type: DioExceptionType.badResponse,
      ));
      expect(e.message, 'Bad email');
      expect(e.statusCode, 400);
    });
  });
}
