package com.rustedbytes.tantivy

import org.json.JSONArray
import org.json.JSONObject

private const val MAX_FIELD_VALUES_PER_DOCUMENT = 10_000
private const val MAX_SEARCH_LIMIT = 1_000
private const val MAX_SEARCH_OFFSET = 100_000
private const val MAX_WRITER_THREADS = 8
private const val MIN_WRITER_MEMORY_BYTES = 15_000_000
private const val MAX_WRITER_MEMORY_BYTES = 512 * 1024 * 1024

enum class FieldType(internal val wireName: String) {
    Text("text"),
    String("string"),
    I64("i64"),
    U64("u64"),
    F64("f64"),
    Bool("bool"),
    Bytes("bytes"),
    ;

    companion object {
        internal fun fromWireName(wireName: String): FieldType =
            entries.firstOrNull { it.wireName == wireName }
                ?: throw NativeLibraryException("Unknown field type: $wireName")
    }
}

enum class TokenizerMode(internal val wireName: String) {
    Default("default"),
    Raw("raw"),
}

data class SchemaField(
    val name: String,
    val type: FieldType,
    val stored: Boolean = true,
    val indexed: Boolean = true,
    val fast: Boolean = false,
    val tokenizer: TokenizerMode? = null,
    val experimental: Boolean = false,
)

class IndexSchema private constructor(
    val fields: List<SchemaField>,
    val defaultSearchFields: List<String>,
) {
    init {
        require(fields.isNotEmpty()) { "At least one schema field is required" }
        require(fields.map { it.name }.toSet().size == fields.size) { "Schema field names must be unique" }
        require(defaultSearchFields.all { fieldName -> fields.any { it.name == fieldName } }) {
            "Default search fields must exist in schema"
        }
    }

    fun toJson(): String = JSONObject()
        .put(
            "fields",
            JSONArray(fields.map { field ->
                JSONObject()
                    .put("name", field.name)
                    .put("type", field.type.wireName)
                    .put("stored", field.stored)
                    .put("indexed", field.indexed)
                    .put("fast", field.fast)
                    .also { json ->
                        field.tokenizer?.let { json.put("tokenizer", it.wireName) }
                        if (field.experimental) json.put("experimental", true)
                    }
            }),
        )
        .put("defaultSearchFields", JSONArray(defaultSearchFields))
        .toString()

    companion object {
        fun build(block: Builder.() -> Unit): IndexSchema = Builder().apply(block).build()
    }

    class Builder {
        private val fields = mutableListOf<SchemaField>()
        private val defaultSearchFields = mutableListOf<String>()

        fun text(
            name: String,
            stored: Boolean = true,
            indexed: Boolean = true,
            defaultSearch: Boolean = true,
            tokenizer: TokenizerMode = TokenizerMode.Default,
        ) {
            field(name, FieldType.Text, stored, indexed, tokenizer = tokenizer)
            if (defaultSearch) defaultSearchFields += name
        }

        fun string(
            name: String,
            stored: Boolean = true,
            indexed: Boolean = true,
            defaultSearch: Boolean = false,
            tokenizer: TokenizerMode = TokenizerMode.Raw,
        ) {
            field(name, FieldType.String, stored, indexed, tokenizer = tokenizer)
            if (defaultSearch) defaultSearchFields += name
        }

        fun i64(name: String, stored: Boolean = true, indexed: Boolean = true, fast: Boolean = false) =
            field(name, FieldType.I64, stored, indexed, fast)

        fun u64(name: String, stored: Boolean = true, indexed: Boolean = true, fast: Boolean = false) =
            field(name, FieldType.U64, stored, indexed, fast)

        fun f64(name: String, stored: Boolean = true, indexed: Boolean = true, fast: Boolean = false) =
            field(name, FieldType.F64, stored, indexed, fast)

        fun bool(name: String, stored: Boolean = true, indexed: Boolean = true, fast: Boolean = false) =
            field(name, FieldType.Bool, stored, indexed, fast)

        fun bytes(name: String, stored: Boolean = true, indexed: Boolean = true, fast: Boolean = false) =
            field(name, FieldType.Bytes, stored, indexed, fast)

        fun field(
            name: String,
            type: FieldType,
            stored: Boolean = true,
            indexed: Boolean = true,
            fast: Boolean = false,
            tokenizer: TokenizerMode? = null,
            experimental: Boolean = false,
        ) {
            require(name.isNotBlank()) { "Field name cannot be blank" }
            fields += SchemaField(name, type, stored, indexed, fast, tokenizer, experimental)
        }

        fun defaultSearchFields(vararg names: String) {
            defaultSearchFields += names
        }

        fun build(): IndexSchema = IndexSchema(fields.toList(), defaultSearchFields.distinct())
    }
}

sealed class FieldValue {
    abstract val type: FieldType
    internal abstract fun rawJsonValue(): Any

    data class Text(val value: kotlin.String) : FieldValue() {
        override val type = FieldType.Text
        override fun rawJsonValue(): Any = value
    }

    data class StringValue(val value: kotlin.String) : FieldValue() {
        override val type = FieldType.String
        override fun rawJsonValue(): Any = value
    }

    data class I64(val value: Long) : FieldValue() {
        override val type = FieldType.I64
        override fun rawJsonValue(): Any = value
    }

    data class U64(val value: Long) : FieldValue() {
        init {
            require(value >= 0) { "U64 value must be non-negative in Kotlin Long representation" }
        }

        override val type = FieldType.U64
        override fun rawJsonValue(): Any = value
    }

    data class F64(val value: Double) : FieldValue() {
        override val type = FieldType.F64
        override fun rawJsonValue(): Any = value
    }

    data class Bool(val value: Boolean) : FieldValue() {
        override val type = FieldType.Bool
        override fun rawJsonValue(): Any = value
    }

    class Bytes(value: ByteArray) : FieldValue() {
        private val bytes: ByteArray = value.copyOf()

        override val type = FieldType.Bytes
        override fun rawJsonValue(): Any = JSONArray(bytes.map { it.toUByte().toInt() })

        fun toByteArray(): ByteArray = bytes.copyOf()
    }
}

data class IndexDocument(val fields: Map<String, List<FieldValue>>) {
    init {
        val valueCount = fields.values.sumOf { it.size }
        require(valueCount <= MAX_FIELD_VALUES_PER_DOCUMENT) {
            "Document cannot contain more than $MAX_FIELD_VALUES_PER_DOCUMENT field values"
        }
    }

    fun toJsonObject(): JSONObject = JSONObject()
        .put(
            "fields",
            JSONObject(fields.mapValues { (_, values) ->
                JSONArray(values.map { value ->
                    JSONObject()
                        .put("type", value.type.wireName)
                        .put("value", value.rawJsonValue())
                })
            }),
        )

    companion object {
        fun build(block: Builder.() -> Unit): IndexDocument = Builder().apply(block).build()
    }

    class Builder {
        private val fields = linkedMapOf<String, MutableList<FieldValue>>()

        fun field(name: String, value: FieldValue) {
            require(name.isNotBlank()) { "Field name cannot be blank" }
            fields.getOrPut(name) { mutableListOf() } += value
        }

        fun text(name: String, value: String) = field(name, FieldValue.Text(value))
        fun string(name: String, value: String) = field(name, FieldValue.StringValue(value))
        fun i64(name: String, value: Long) = field(name, FieldValue.I64(value))
        fun u64(name: String, value: Long) = field(name, FieldValue.U64(value))
        fun f64(name: String, value: Double) = field(name, FieldValue.F64(value))
        fun bool(name: String, value: Boolean) = field(name, FieldValue.Bool(value))
        fun bytes(name: String, value: ByteArray) = field(name, FieldValue.Bytes(value))
        fun put(name: String, value: FieldValue) = field(name, value)
        fun putAll(values: Map<String, FieldValue>) = values.forEach { (name, value) -> field(name, value) }
        fun repeated(name: String, values: Iterable<FieldValue>) = values.forEach { field(name, it) }

        fun build(): IndexDocument = IndexDocument(fields.mapValues { it.value.toList() })
    }
}

data class IndexOptions(
    val create: Boolean = true,
    val writerThreads: Int = 1,
    val writerMemoryBytes: Int = 50_000_000,
    val dispatcher: kotlinx.coroutines.CoroutineDispatcher = kotlinx.coroutines.Dispatchers.IO,
) {
    init {
        require(writerThreads in 1..MAX_WRITER_THREADS) {
            "writerThreads must be between 1 and $MAX_WRITER_THREADS"
        }
        require(writerMemoryBytes in MIN_WRITER_MEMORY_BYTES..MAX_WRITER_MEMORY_BYTES) {
            "writerMemoryBytes must be between $MIN_WRITER_MEMORY_BYTES and $MAX_WRITER_MEMORY_BYTES"
        }
    }

    fun toJson(): String = JSONObject()
        .put("create", create)
        .put("writerThreads", writerThreads)
        .put("writerMemoryBytes", writerMemoryBytes)
        .toString()
}

data class BatchOptions(
    val maxBatchSize: Int = 500,
    val commitEveryBatch: Boolean = false,
    val commitPolicy: CommitPolicy = if (commitEveryBatch) CommitPolicy.EveryBatch else CommitPolicy.Manual,
    val refreshPolicy: RefreshPolicy = RefreshPolicy.Manual,
    val errorPolicy: BatchErrorPolicy = BatchErrorPolicy.Stop,
    val progressGranularity: ProgressGranularity = ProgressGranularity.Batch,
) {
    init {
        require(maxBatchSize > 0) { "maxBatchSize must be positive" }
    }
}

enum class CommitPolicy {
    Manual,
    EveryBatch,
    End,
}

enum class RefreshPolicy {
    Manual,
    AfterCommit,
    End,
}

enum class BatchErrorPolicy {
    Stop,
    Continue,
}

enum class ProgressGranularity {
    Batch,
    CompletionOnly,
}

data class SearchFlowOptions(
    val debounceMillis: Long = 250,
    val distinctUntilChanged: Boolean = true,
    val emitLoading: Boolean = true,
)

enum class SortOrder(internal val wireName: String) {
    Asc("asc"),
    Desc("desc"),
}

data class SortRequest(
    val field: String,
    val order: SortOrder = SortOrder.Desc,
) {
    fun toJsonObject(): JSONObject = JSONObject()
        .put("field", field)
        .put("order", order.wireName)
}

data class SearchRequest(
    val query: String,
    val limit: Int = 20,
    val offset: Int = 0,
    val defaultFields: List<String> = emptyList(),
    val selectedFields: List<String> = emptyList(),
    val sort: SortRequest? = null,
    val reloadBeforeSearch: Boolean = false,
    val flowOptions: SearchFlowOptions = SearchFlowOptions(),
) {
    init {
        require(limit >= 0) { "limit must be non-negative" }
        require(offset >= 0) { "offset must be non-negative" }
        require(limit <= MAX_SEARCH_LIMIT) { "limit must be <= $MAX_SEARCH_LIMIT" }
        require(offset <= MAX_SEARCH_OFFSET) { "offset must be <= $MAX_SEARCH_OFFSET" }
    }

    fun toJson(): String = JSONObject()
        .put("query", query)
        .put("limit", limit)
        .put("offset", offset)
        .put("defaultFields", JSONArray(defaultFields))
        .put("selectedFields", JSONArray(selectedFields))
        .put("reloadBeforeSearch", reloadBeforeSearch)
        .also { json -> sort?.let { json.put("sort", it.toJsonObject()) } }
        .toString()
}

typealias SearchQuery = SearchRequest

data class AdvancedSearchRequest(
    val request: SearchRequest,
)

data class WriteResult(val documentsAdded: Int)
data class DeleteResult(val termsDeleted: Int)
data class CommitResult(val opstamp: Long)
data class RefreshResult(val refreshed: Boolean)
data class CommitRefreshResult(val opstamp: Long, val refreshed: Boolean)
data class SearchPage(val totalHits: Int, val hits: List<SearchHit>)
data class SearchHit(val score: Float, val fields: Map<String, List<FieldValue>>)
data class SchemaInfo(val fields: List<SchemaField>, val defaultSearchFields: List<String>)

interface DocumentMapper<T> {
    fun toDocument(value: T): IndexDocument
    fun fromHit(hit: SearchHit): T? = null
}

sealed class SearchState {
    data object Loading : SearchState()
    data class Success(val page: SearchPage) : SearchState()
    data object Empty : SearchState()
    data class Error(val error: TantivyException) : SearchState()
}

sealed class IndexState {
    data object Opening : IndexState()
    data object Open : IndexState()
    data object Committing : IndexState()
    data object Refreshing : IndexState()
    data object Closed : IndexState()
    data class Error(val error: TantivyException) : IndexState()
}

sealed class IndexingProgress {
    data class Batch(val documentsIndexed: Int, val totalIndexed: Int) : IndexingProgress()
    data class Complete(val totalIndexed: Int) : IndexingProgress()
}
