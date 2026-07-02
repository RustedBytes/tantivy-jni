package com.rustedbytes.tantivy

internal interface NativeBridge {
    fun openIndex(path: String, schemaJson: String, optionsJson: String): Long
    fun closeIndex(handle: Long)
    fun addDocuments(handle: Long, documentsJson: String): String
    fun deleteTerm(handle: Long, field: String, valueJson: String): String
    fun commit(handle: Long): String
    fun refresh(handle: Long): String
    fun commitAndRefresh(handle: Long): String
    fun schemaInfo(handle: Long): String
    fun search(handle: Long, queryJson: String): String
}

internal object JniNativeBridge : NativeBridge {
    init {
        try {
            System.loadLibrary("tantivy_jni")
        } catch (error: UnsatisfiedLinkError) {
            throw NativeLibraryException("Unable to load tantivy_jni native library", error)
        }
    }

    override fun openIndex(path: String, schemaJson: String, optionsJson: String): Long =
        NativeTantivy.nativeOpenIndex(path, schemaJson, optionsJson)

    override fun closeIndex(handle: Long) = NativeTantivy.nativeCloseIndex(handle)

    override fun addDocuments(handle: Long, documentsJson: String): String =
        NativeTantivy.nativeAddDocuments(handle, documentsJson)

    override fun deleteTerm(handle: Long, field: String, valueJson: String): String =
        NativeTantivy.nativeDeleteTerm(handle, field, valueJson)

    override fun commit(handle: Long): String = NativeTantivy.nativeCommit(handle)

    override fun refresh(handle: Long): String = NativeTantivy.nativeRefresh(handle)

    override fun commitAndRefresh(handle: Long): String = NativeTantivy.nativeCommitAndRefresh(handle)

    override fun schemaInfo(handle: Long): String = NativeTantivy.nativeSchemaInfo(handle)

    override fun search(handle: Long, queryJson: String): String =
        NativeTantivy.nativeSearch(handle, queryJson)
}

internal object NativeTantivy {
    external fun nativeOpenIndex(path: String, schemaJson: String, optionsJson: String): Long
    external fun nativeCloseIndex(handle: Long)
    external fun nativeAddDocuments(handle: Long, documentsJson: String): String
    external fun nativeDeleteTerm(handle: Long, field: String, valueJson: String): String
    external fun nativeCommit(handle: Long): String
    external fun nativeRefresh(handle: Long): String
    external fun nativeCommitAndRefresh(handle: Long): String
    external fun nativeSchemaInfo(handle: Long): String
    external fun nativeSearch(handle: Long, queryJson: String): String
}
