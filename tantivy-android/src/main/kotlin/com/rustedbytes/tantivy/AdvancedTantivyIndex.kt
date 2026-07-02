package com.rustedbytes.tantivy

@AdvancedTantivyApi
class AdvancedTantivyIndex internal constructor(
    private val index: TantivyIndex,
) {
    suspend fun nativeSearch(request: AdvancedSearchRequest): SearchPage =
        index.search(request.request)

    @OptIn(AdvancedTantivyApi::class)
    suspend fun commitAndRefresh(): CommitRefreshResult =
        index.commitAndRefresh()

    @OptIn(AdvancedTantivyApi::class)
    suspend fun schemaInfo(): SchemaInfo =
        index.schemaInfo()
}
