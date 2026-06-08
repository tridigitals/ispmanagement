# Flutter
-keep class io.flutter.app.** { *; }
-keep class io.flutter.plugin.** { *; }
-keep class io.flutter.util.** { *; }
-keep class io.flutter.view.** { *; }
-keep class io.flutter.** { *; }
-keep class io.flutter.plugins.** { *; }
-keep class io.flutter.embedding.** { *; }

# Play Core split install (Flutter deferred components) — referenced but not used
-dontwarn com.google.android.play.core.**
-keep class com.google.android.play.core.** { *; }

# Dio
-keep class io.flutter.plugins.** { *; }

# JSON serialization
-keepattributes *Annotation*
-keepattributes Signature
-keep class **$JsonObject { *; }
-keep class com.google.gson.** { *; }

# Keep model classes (used with json_serializable)
-keep class com.tridigitals.ispcustomer.** { *; }

# Sentry — keep class names + line numbers for stack trace deobfuscation
-keepattributes SourceFile,LineNumberTable
-renamesourcefileattribute SourceFile
-keep class io.sentry.** { *; }
-keep class io.sentry.android.** { *; }
-keep class io.sentry.flutter.** { *; }
-dontwarn io.sentry.**

# Firebase — keep all Firebase classes (FCM, Core, Messaging)
-keep class com.google.firebase.** { *; }
-keep class io.flutter.plugins.firebase.** { *; }
-keep class com.google.android.gms.** { *; }
-dontwarn com.google.firebase.**
-dontwarn com.google.android.gms.**

# flutter_cache_manager
-keep class com.example.flutter_cache_manager_example.** { *; }
