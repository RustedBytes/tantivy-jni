package com.rustedbytes.tantivy

open class TantivyException(message: String, cause: Throwable? = null) : RuntimeException(message, cause)

class SchemaException(message: String, cause: Throwable? = null) : TantivyException(message, cause)

class IndexOpenException(message: String, cause: Throwable? = null) : TantivyException(message, cause)

class WriteException(message: String, cause: Throwable? = null) : TantivyException(message, cause)

class SearchException(message: String, cause: Throwable? = null) : TantivyException(message, cause)

class NativeLibraryException(message: String, cause: Throwable? = null) : TantivyException(message, cause)

class TantivyIndexClosedException(message: String = "Tantivy index is closed") : TantivyException(message)
