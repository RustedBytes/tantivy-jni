# JNI exports use the concrete JVM class and method names in NativeTantivy.
# Keep this bridge stable when consuming apps enable R8/ProGuard minification.
-keep class com.rustedbytes.tantivy.NativeTantivy {
    *;
}
