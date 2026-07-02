package com.rustedbytes.tantivy

import org.json.JSONArray
import org.json.JSONObject

private const val MAX_SEARCH_LIMIT = 1_000
private const val MAX_SEARCH_OFFSET = 100_000

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
