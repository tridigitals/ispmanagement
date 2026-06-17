1|import 'dart:async';
2|
3|import 'package:firebase_core/firebase_core.dart';
4|import 'package:firebase_messaging/firebase_messaging.dart';
5|import 'package:flutter/material.dart';
6|import 'package:flutter_local_notifications/flutter_local_notifications.dart';
7|import 'package:flutter_riverpod/flutter_riverpod.dart';
8|import 'package:go_router/go_router.dart';
9|
10|import 'app_config.dart';
11|
12|/// Background handler — must be a top-level function.
13|///
14|/// Called when the app receives a message while in the background or
15|/// terminated. For data-only messages we must display a local notification
16|/// ourselves because Android won't show anything automatically.
17|@pragma('vm:entry-point')
18|Future<void> firebaseMessagingBackgroundHandler(RemoteMessage message) async {
19|  await Firebase.initializeApp();
20|  debugPrint('[FCM] Background message: ${message.messageId}');
21|
22|  // Only show local notification for data-only messages (no notification payload).
23|  // Messages with a `notification` field are displayed automatically by Android.
24|  final data = message.data;
25|  final title = data['title'] as String? ?? 'ISP Customer';
26|  final body = data['body'] as String? ?? '';
27|
28|  if (message.notification == null && body.isNotEmpty) {
29|    final localNotif = FlutterLocalNotificationsPlugin();
30|    const androidSettings =
31|        AndroidInitializationSettings('@mipmap/ic_launcher');
32|    const initSettings = InitializationSettings(android: androidSettings);
33|    await localNotif.initialize(initSettings);
34|
35|    await localNotif.show(
36|      message.hashCode,
37|      title,
38|      body,
39|      const NotificationDetails(
40|        android: AndroidNotificationDetails(
41|          'high_importance_channel',
42|          'Notifikasi Penting',
43|          channelDescription: 'Tagihan, gangguan, dan info langganan ISP',
44|          importance: Importance.high,
45|          priority: Priority.high,
46|          icon: '@mipmap/ic_launcher',
47|        ),
48|      ),
49|      payload: data['action_url'] as String?,
50|    );
51|  }
52|}
53|
54|/// Android notification channel for high-importance messages.
55|const AndroidNotificationChannel _channel = AndroidNotificationChannel(
56|  'high_importance_channel',
57|  'Notifikasi Penting',
58|  description: 'Tagihan, gangguan, dan info langganan ISP',
59|  importance: Importance.high,
60|  playSound: true,
61|  enableVibration: true,
62|);
63|
64|final FlutterLocalNotificationsPlugin _localNotif =
65|    FlutterLocalNotificationsPlugin();
66|
67|/// Singleton service managing FCM lifecycle.
68|class FcmService {
69|  FcmService(this._ref);
70|  final Ref _ref;
71|  bool _initialized = false;
72|  bool _inFlight = false;
73|
74|  /// Visible status for debug banner.
75|  void _status(String msg) {
76|    debugPrint('[FCM] $msg');
77|    _ref.read(fcmStatusProvider.notifier).state = msg;
78|  }
79|
80|  /// Initialize FCM — call after every successful login AND on every app start.
81|  ///
82|  /// Why both: a cold start with a still-valid session doesn't transition
83|  /// through an auth state change (state goes from null→authenticated during
84|  /// bootstrap, but timing is fragile). On top of that, FCM tokens can be
85|  /// invalidated by Firebase at any time (uninstall, app data clear, 270-day
86|  /// expiry, etc.) so we re-register on every app open.
87|  ///
88|  /// Robustness: every step has a timeout, and failures are caught locally.
89|  /// The whole init is fire-and-forget — callers do NOT await.
90|  Future<void> init({bool force = false}) async {
91|    if (_inFlight) return; // already running
92|    if (_initialized && !force) return;
93|    _inFlight = true;
94|    // NOTE: we DON'T set _initialized=true here. Setting it on success lets
95|    // a failed init get retried on the next call.
96|    _status('init() running (force=$force)');
97|
98|    try {
99|      _status('Step 1/6: _initInternal starting');
100|      await _initInternal();
101|      _initialized = true;
102|      _status('✅ init OK — all steps completed');
103|    } catch (e, st) {
104|      _status('❌ init error: $e');
105|    } finally {
106|      _inFlight = false;
107|    }
108|  }
109|
110|  Future<void> _initInternal() async {
111|    _status('Step 1/6: LocalNotification init');
112|    // Local notification init — fast.
113|    const androidSettings =
114|        AndroidInitializationSettings('@mipmap/ic_launcher');
115|    const initSettings = InitializationSettings(android: androidSettings);
116|    await _localNotif.initialize(
117|      initSettings,
118|      onDidReceiveNotificationResponse: _onLocalNotifTap,
119|    );
120|
121|    // Create Android channel.
122|    await _localNotif
123|        .resolvePlatformSpecificImplementation<
124|            AndroidFlutterLocalNotificationsPlugin>()
125|        ?.createNotificationChannel(_channel);
126|    _status('Step 2/6: NotificationChannel created');
127|
128|    // Request permission (Android 13+). Wrapped in 5s timeout — this call
129|    // can hang on some devices if the system dialog is interrupted.
130|    _status('Step 3/6: Requesting permission...');
131|    final settings = await FirebaseMessaging.instance
132|        .requestPermission(
133|          alert: true,
134|          badge: true,
135|          sound: true,
136|          provisional: false,
137|        )
138|        .timeout(const Duration(seconds: 5), onTimeout: () {
139|      _status('⚠️ requestPermission timeout — assuming granted');
140|      // Return a default-allowed value so init continues. If the user did
141|      // deny, the actual FCM call will still fail downstream.
142|      return NotificationSettings(
143|        alert: AppleNotificationSetting.enabled,
144|        announcement: AppleNotificationSetting.enabled,
145|        authorizationStatus: AuthorizationStatus.authorized,
146|        badge: AppleNotificationSetting.enabled,
147|        carPlay: AppleNotificationSetting.enabled,
148|        lockScreen: AppleNotificationSetting.enabled,
149|        notificationCenter: AppleNotificationSetting.enabled,
150|        showPreviews: AppleShowPreviewSetting.always,
151|        timeSensitive: AppleNotificationSetting.enabled,
152|        criticalAlert: AppleNotificationSetting.disabled,
153|        sound: AppleNotificationSetting.enabled,
154|        providesAppNotificationSettings: AppleNotificationSetting.disabled,
155|      );
156|    });
157|    _status('Permission: ${settings.authorizationStatus}');
158|
159|    _status('Step 4/6: Setting up listeners');
160|    // Listen foreground messages.
161|    FirebaseMessaging.onMessage.listen(_onForegroundMessage);
162|
163|    // Listen tap when app in background.
164|    FirebaseMessaging.onMessageOpenedApp.listen(_onMessageTapped);
165|
166|    // Cold start — check if opened from notification. 3s timeout: this is
167|    // best-effort and shouldn't block app startup.
168|    _status('Step 5/6: getInitialMessage');
169|    try {
170|      final initial = await FirebaseMessaging.instance
171|          .getInitialMessage()
172|          .timeout(const Duration(seconds: 3));
173|      if (initial != null) {
174|        WidgetsBinding.instance.addPostFrameCallback((_) {
175|          _onMessageTapped(initial);
176|        });
177|      }
178|    } catch (e) {
179|      _status('⚠️ getInitialMessage failed: $e');
180|    }
181|
182|    // Register token — with built-in retry on null/error.
183|    _status('Step 6/6: Registering token...');
184|    await _registerTokenWithRetry();
185|
186|    // Listen for token refresh (e.g., Firebase rotates the token).
187|    FirebaseMessaging.instance.onTokenRefresh.listen((_) {
188|      _status('Token refresh — re-registering');
189|      // ignore: discarded_futures
190|      _registerTokenWithRetry();
191|    });
192|  }
193|
194|  /// Register FCM token with backend, with 1 retry on null/error.
195|  Future<void> _registerTokenWithRetry() async {
196|    for (var attempt = 1; attempt <= 2; attempt++) {
197|      try {
198|        final ok = await _registerToken();
199|        if (ok) return;
200|        // Token was null — wait 2s and retry (Firebase may need a moment
201|        // to provision a token after first install).
202|        if (attempt == 1) {
203|          _status('No token yet, retrying in 2s…');
204|          await Future.delayed(const Duration(seconds: 2));
205|        }
206|      } catch (e) {
207|        _status('❌ Register attempt $attempt: $e');
208|        if (attempt == 1) {
209|          await Future.delayed(const Duration(seconds: 2));
210|        }
211|      }
212|    }
213|    _status('❌ All register attempts failed');
214|  }
215|
216|  /// Returns true on success, false on null token, throws on transport error.
217|  Future<bool> _registerToken() async {
218|    final token = await FirebaseMessaging.instance
219|        .getToken()
220|        .timeout(const Duration(seconds: 10), onTimeout: () {
221|      _status('❌ getToken timeout (10s)');
222|      return null;
223|    });
224|    if (token == null) {
225|      _status('⚠️ Token is null');
226|      return false;
227|    }
228|    _status('Token: ${token.substring(0, 20)}…');
229|
230|    final dio = _ref.read(dioProvider);
231|    await dio
232|        .post(
233|          '/api/notifications/devices',
234|          data: {
235|            'fcm_token': token,
236|            'platform': 'android',
237|          },
238|        )
239|        .timeout(const Duration(seconds: 10));
240|    _status('✅ Token registered OK!');
241|    return true;
242|  }
243|
244|  /// Foreground message — show local notification.
245|  void _onForegroundMessage(RemoteMessage message) {
246|    final notif = message.notification;
247|    if (notif == null) return;
248|
249|    debugPrint('[FCM] Foreground: ${notif.title}');
250|
251|    _localNotif.show(
252|      message.hashCode,
253|      notif.title,
254|      notif.body,
255|      NotificationDetails(
256|        android: AndroidNotificationDetails(
257|          _channel.id,
258|          _channel.name,
259|          channelDescription: _channel.description,
260|          importance: Importance.high,
261|          priority: Priority.high,
262|          icon: '@mipmap/ic_launcher',
263|          color: const Color(0xFF6C63FF),
264|        ),
265|      ),
266|      payload: message.data['action_url'] as String?,
267|    );
268|  }
269|
270|  /// Handle local notification tap (foreground).
271|  void _onLocalNotifTap(NotificationResponse response) {
272|    final actionUrl = response.payload;
273|    _navigateToAction(actionUrl);
274|  }
275|
276|  /// Handle FCM message tap (background / cold start).
277|  void _onMessageTapped(RemoteMessage message) {
278|    final actionUrl = message.data['action_url'] as String?;
279|    _navigateToAction(actionUrl);
280|  }
281|
282|  /// Normalize action_url → in-app route and navigate.
283|  void _navigateToAction(String? actionUrl) {
284|    try {
285|      final navKey = _ref.read(navigatorKeyProvider);
286|      final context = navKey.currentContext;
287|      if (context == null) return;
288|
289|      String route;
290|      if (actionUrl == null || actionUrl.isEmpty) {
291|        route = '/notifications';
292|      } else if (actionUrl.startsWith('/support/')) {
293|        route = '/?tab=3';
294|      } else if (actionUrl.startsWith('/pay/') ||
295|          actionUrl.startsWith('/invoices')) {
296|        route = '/?tab=2';
297|      } else if (actionUrl.startsWith('/services') ||
298|          actionUrl.startsWith('/subscriptions/')) {
299|        route = '/?tab=1';
300|      } else if (actionUrl.startsWith('/announcements/')) {
301|        route = '/?tab=0';
302|      } else {
303|        route = actionUrl;
304|      }
305|
306|      GoRouter.of(context).go(route);
307|    } catch (e) {
308|      debugPrint('[FCM] Navigation error: $e');
309|    }
310|  }
311|}
312|
313|/// Riverpod provider for FCM service.
314|/// Debug status visible on home screen banner.
final fcmStatusProvider = StateProvider<String>((ref) => 'Waiting…');

final fcmServiceProvider = Provider<FcmService>((ref) {
315|  return FcmService(ref);
316|});
317|
318|/// Global navigator key — shared with GoRouter.
319|final navigatorKeyProvider = Provider<GlobalKey<NavigatorState>>((ref) {
320|  return GlobalKey<NavigatorState>();
321|});
322|