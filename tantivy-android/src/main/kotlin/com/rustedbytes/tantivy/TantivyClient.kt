package com.rustedbytes.tantivy

object TantivyClient {
    suspend fun open(
        path: String,
        schema: IndexSchema,
        options: IndexOptions = IndexOptions(),
    ): TantivyIndex = TantivyIndex.open(path, schema, options)

    fun schema(block: IndexSchema.Builder.() -> Unit): IndexSchema = IndexSchema.build(block)

    fun document(block: IndexDocument.Builder.() -> Unit): IndexDocument = IndexDocument.build(block)

    fun query(block: SearchRequestBuilder.() -> Unit): SearchRequest =
        SearchRequestBuilder().apply(block).build()
}

class SearchRequestBuilder {
    var query: String = ""
    var limit: Int = 20
    var offset: Int = 0
    private val defaultFields = mutableListOf<String>()
    private val selectedFields = mutableListOf<String>()
    private var sort: SortRequest? = null
    var reloadBeforeSearch: Boolean = false
    var flowOptions: SearchFlowOptions = SearchFlowOptions()

    fun defaultFields(vararg names: String) {
        defaultFields += names
    }

    fun selectedFields(vararg names: String) {
        selectedFields += names
    }

    fun sortBy(field: String, order: SortOrder = SortOrder.Desc) {
        sort = SortRequest(field, order)
    }

    fun build(): SearchRequest = SearchRequest(
        query = query,
        limit = limit,
        offset = offset,
        defaultFields = defaultFields.distinct(),
        selectedFields = selectedFields.distinct(),
        sort = sort,
        reloadBeforeSearch = reloadBeforeSearch,
        flowOptions = flowOptions,
    )
}

@RequiresOptIn(level = RequiresOptIn.Level.WARNING)
annotation class AdvancedTantivyApi
