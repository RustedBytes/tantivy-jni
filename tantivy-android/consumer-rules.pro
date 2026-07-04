# JNI exports use the concrete JVM class and method names in NativeTantivy.
# Keep this bridge stable when consuming apps enable R8/ProGuard minification.
-keep class com.rustedbytes.tantivy.NativeTantivy {
    *;
}

# Native code throws these exceptions by looking up the classes by name via
# JNI FindClass, so they must survive shrinking and keep their original names.
-keep class com.rustedbytes.tantivy.TantivyException { *; }
-keep class com.rustedbytes.tantivy.SchemaException { *; }
-keep class com.rustedbytes.tantivy.IndexOpenException { *; }
-keep class com.rustedbytes.tantivy.WriteException { *; }
-keep class com.rustedbytes.tantivy.SearchException { *; }
-keep class com.rustedbytes.tantivy.NativeLibraryException { *; }
-keep class com.rustedbytes.tantivy.TantivyIndexClosedException { *; }
