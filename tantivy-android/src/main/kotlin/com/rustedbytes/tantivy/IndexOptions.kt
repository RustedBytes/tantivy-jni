package com.rustedbytes.tantivy

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import org.json.JSONObject

private const val MAX_WRITER_THREADS = 8
private const val MIN_WRITER_MEMORY_BYTES = 15_000_000
private const val MAX_WRITER_MEMORY_BYTES = 512 * 1024 * 1024

data class IndexOptions(
    val create: Boolean = true,
    val writerThreads: Int = 1,
    val writerMemoryBytes: Int = 50_000_000,
    val dispatcher: CoroutineDispatcher = Dispatchers.IO,
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
