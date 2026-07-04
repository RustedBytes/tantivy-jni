package com.rustedbytes.tantivy

open class TantivyException @JvmOverloads constructor(message: String, cause: Throwable? = null) : RuntimeException(message, cause)

class SchemaException @JvmOverloads constructor(message: String, cause: Throwable? = null) : TantivyException(message, cause)

class IndexOpenException @JvmOverloads constructor(message: String, cause: Throwable? = null) : TantivyException(message, cause)

class WriteException @JvmOverloads constructor(message: String, cause: Throwable? = null) : TantivyException(message, cause)

class SearchException @JvmOverloads constructor(message: String, cause: Throwable? = null) : TantivyException(message, cause)

class NativeLibraryException @JvmOverloads constructor(message: String, cause: Throwable? = null) : TantivyException(message, cause)

class TantivyIndexClosedException @JvmOverloads constructor(message: String = "Tantivy index is closed") : TantivyException(message)
