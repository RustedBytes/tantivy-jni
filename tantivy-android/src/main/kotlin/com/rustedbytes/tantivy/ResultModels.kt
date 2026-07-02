package com.rustedbytes.tantivy

data class WriteResult(val documentsAdded: Int)
data class DeleteResult(val termsDeleted: Int)
data class CommitResult(val opstamp: Long)
data class RefreshResult(val refreshed: Boolean)
data class CommitRefreshResult(val opstamp: Long, val refreshed: Boolean)
data class SearchPage(val totalHits: Int, val hits: List<SearchHit>)
data class SearchHit(val score: Float, val fields: Map<String, List<FieldValue>>, val snippets: Map<String, String> = emptyMap())
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
