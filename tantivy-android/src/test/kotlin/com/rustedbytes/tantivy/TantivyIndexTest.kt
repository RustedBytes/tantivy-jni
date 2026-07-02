package com.rustedbytes.tantivy

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlinx.coroutines.awaitCancellation
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.flow
import kotlinx.coroutines.flow.asFlow
import kotlinx.coroutines.flow.toList
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.json.JSONArray
import org.json.JSONObject

@OptIn(ExperimentalCoroutinesApi::class)
class TantivyIndexTest {
    @Test
    fun schemaBuilderSerializesFlexibleOptions() {
        val schema = TantivyClient.schema {
            text("title", tokenizer = TokenizerMode.Default)
            string("sku", tokenizer = TokenizerMode.Raw)
            field(
                name = "rank",
                type = FieldType.I64,
                stored = false,
                indexed = true,
                fast = true,
                experimental = true,
            )
            defaultSearchFields("title")
        }

        val json = JSONObject(schema.toJson())
        val fields = json.getJSONArray("fields")
        val title = fields.getJSONObject(0)
        val sku = fields.getJSONObject(1)
        val rank = fields.getJSONObject(2)

        assertEquals("default", title.getString("tokenizer"))
        assertEquals("raw", sku.getString("tokenizer"))
        assertEquals(false, rank.getBoolean("stored"))
        assertEquals(true, rank.getBoolean("fast"))
        assertEquals(true, rank.getBoolean("experimental"))
        assertEquals("title", json.getJSONArray("defaultSearchFields").getString(0))
    }

    @Test
    fun searchRequestBuilderSerializesAdvancedOptions() {
        val request = TantivyClient.query {
            query = "android"
            limit = 5
            offset = 2
            defaultFields("title")
            selectedFields("id", "price")
            sortBy("price", SortOrder.Asc)
            reloadBeforeSearch = true
        }

        val json = JSONObject(request.toJson())
        assertEquals("android", json.getString("query"))
        assertEquals("price", json.getJSONObject("sort").getString("field"))
        assertEquals("asc", json.getJSONObject("sort").getString("order"))
        assertEquals(true, json.getBoolean("reloadBeforeSearch"))
        assertEquals("id", json.getJSONArray("selectedFields").getString(0))
    }

    @Test
    fun searchRequestRejectsInvalidPagination() {
        assertFailsWith<IllegalArgumentException> {
            SearchRequest(query = "android", limit = 1_001)
        }
        assertFailsWith<IllegalArgumentException> {
            SearchRequest(query = "android", offset = 100_001)
        }
    }

    @Test
    fun documentBuilderSupportsPutAllAndRepeatedValues() {
        val document = TantivyClient.document {
            putAll(
                mapOf(
                    "id" to FieldValue.StringValue("doc-1"),
                    "published" to FieldValue.Bool(true),
                ),
            )
            repeated(
                "tag",
                listOf(
                    FieldValue.StringValue("android"),
                    FieldValue.StringValue("search"),
                ),
            )
        }

        assertEquals(1, document.fields["id"]?.size)
        assertEquals(2, document.fields["tag"]?.size)
    }

    @Test
    fun mapperCanRoundTripDocumentAndHit() {
        data class Article(val id: String, val title: String)
        val mapper = object : DocumentMapper<Article> {
            override fun toDocument(value: Article): IndexDocument = TantivyClient.document {
                string("id", value.id)
                text("title", value.title)
            }

            override fun fromHit(hit: SearchHit): Article? {
                val id = (hit.fields["id"]?.firstOrNull() as? FieldValue.StringValue)?.value ?: return null
                val title = (hit.fields["title"]?.firstOrNull() as? FieldValue.Text)?.value ?: return null
                return Article(id, title)
            }
        }

        val document = mapper.toDocument(Article("1", "hello"))
        val hit = SearchHit(score = 1f, fields = document.fields)
        assertEquals(Article("1", "hello"), mapper.fromHit(hit))
    }

    @Test
    fun closedIndexRejectsOperations() = runTest {
        val bridge = FakeBridge()
        val index = TantivyIndex.fromNativeHandle(1, IndexOptions(dispatcher = UnconfinedTestDispatcher(testScheduler)), bridge)
        index.close()

        assertFailsWith<TantivyIndexClosedException> {
            index.add(IndexDocument.build { text("title", "value") })
        }
    }

    @Test
    fun indexDocumentsEmitsBatchProgressAndCompletion() = runTest {
        val bridge = FakeBridge()
        val index = TantivyIndex.fromNativeHandle(1, IndexOptions(dispatcher = UnconfinedTestDispatcher(testScheduler)), bridge)
        val documents = listOf(
            IndexDocument.build { text("title", "one") },
            IndexDocument.build { text("title", "two") },
            IndexDocument.build { text("title", "three") },
        )

        val emissions = index
            .indexDocuments(documents.asFlow(), BatchOptions(maxBatchSize = 2))
            .toList()

        assertEquals(
            listOf(
                IndexingProgress.Batch(documentsIndexed = 2, totalIndexed = 2),
                IndexingProgress.Batch(documentsIndexed = 1, totalIndexed = 3),
                IndexingProgress.Complete(totalIndexed = 3),
            ),
            emissions,
        )
    }

    @Test
    fun indexDocumentsCommitsAndRefreshesAtEnd() = runTest {
        val bridge = FakeBridge()
        val index = TantivyIndex.fromNativeHandle(1, IndexOptions(dispatcher = UnconfinedTestDispatcher(testScheduler)), bridge)
        val documents = listOf(
            IndexDocument.build { text("title", "one") },
            IndexDocument.build { text("title", "two") },
            IndexDocument.build { text("title", "three") },
        )

        val emissions = index
            .indexDocuments(
                documents.asFlow(),
                BatchOptions(
                    maxBatchSize = 2,
                    commitPolicy = CommitPolicy.End,
                    refreshPolicy = RefreshPolicy.AfterCommit,
                    progressGranularity = ProgressGranularity.CompletionOnly,
                ),
            )
            .toList()

        assertEquals(listOf(IndexingProgress.Complete(totalIndexed = 3)), emissions)
        assertEquals(1, bridge.commitCalls)
        assertEquals(1, bridge.refreshCalls)
    }

    @Test
    fun indexDocumentsContinuesAfterFailedBatchWhenConfigured() = runTest {
        val bridge = FakeBridge(failingAddCalls = mutableSetOf(1))
        val index = TantivyIndex.fromNativeHandle(1, IndexOptions(dispatcher = UnconfinedTestDispatcher(testScheduler)), bridge)
        val documents = listOf(
            IndexDocument.build { text("title", "failed") },
            IndexDocument.build { text("title", "indexed") },
        )

        val emissions = index
            .indexDocuments(
                documents.asFlow(),
                BatchOptions(
                    maxBatchSize = 1,
                    errorPolicy = BatchErrorPolicy.Continue,
                ),
            )
            .toList()

        assertEquals(
            listOf(
                IndexingProgress.Batch(documentsIndexed = 1, totalIndexed = 1),
                IndexingProgress.Complete(totalIndexed = 1),
            ),
            emissions,
        )
        assertEquals(2, bridge.addCalls)
    }

    @Test
    fun launchIndexingStopsCollectingWhenCancelledBetweenBatches() = runTest {
        val bridge = FakeBridge()
        val index = TantivyIndex.fromNativeHandle(1, IndexOptions(dispatcher = UnconfinedTestDispatcher(testScheduler)), bridge)
        val documents = flow {
            emit(IndexDocument.build { text("title", "one") })
            awaitCancellation()
            emit(IndexDocument.build { text("title", "two") })
        }

        val job = index.launchIndexing(
            scope = this,
            documents = documents,
            options = BatchOptions(maxBatchSize = 1),
        )
        advanceUntilIdle()
        assertEquals(1, bridge.addCalls)
        job.cancel()
        advanceUntilIdle()

        assertEquals(1, bridge.addCalls)
    }

    @Test
    fun searchFlowUsesLatestQuery() = runTest {
        val dispatcher = StandardTestDispatcher(testScheduler)
        val bridge = FakeBridge()
        val index = TantivyIndex.fromNativeHandle(1, IndexOptions(dispatcher = dispatcher), bridge)
        val queries = MutableSharedFlow<SearchQuery>(extraBufferCapacity = 2)
        val states = mutableListOf<SearchState>()

        val job = launch {
            index.searchFlow(queries).toList(states)
        }
        advanceUntilIdle()

        queries.emit(SearchQuery("old"))
        queries.emit(SearchQuery("new"))
        advanceTimeBy(300)
        advanceUntilIdle()

        assertEquals("new", bridge.lastSearchQuery)
        job.cancel()
    }

    @Test
    fun searchFlowCanSkipLoadingAndEmitEmpty() = runTest {
        val dispatcher = StandardTestDispatcher(testScheduler)
        val bridge = FakeBridge(searchHits = JSONArray())
        val index = TantivyIndex.fromNativeHandle(1, IndexOptions(dispatcher = dispatcher), bridge)
        val queries = MutableSharedFlow<SearchRequest>(extraBufferCapacity = 1)
        val states = mutableListOf<SearchState>()

        val job = launch {
            index.searchFlow(queries).toList(states)
        }
        advanceUntilIdle()

        queries.emit(
            SearchRequest(
                query = "missing",
                flowOptions = SearchFlowOptions(debounceMillis = 0, emitLoading = false),
            ),
        )
        advanceUntilIdle()

        assertEquals(listOf(SearchState.Empty), states)
        job.cancel()
    }

    @OptIn(AdvancedTantivyApi::class)
    @Test
    fun advancedApiCallsBridgeMethods() = runTest {
        val bridge = FakeBridge()
        val index = TantivyIndex.fromNativeHandle(1, IndexOptions(dispatcher = UnconfinedTestDispatcher(testScheduler)), bridge)

        assertEquals(true, index.advanced.commitAndRefresh().refreshed)
        assertEquals("title", index.advanced.schemaInfo().fields.first().name)
        index.advanced.nativeSearch(
            AdvancedSearchRequest(
                TantivyClient.query {
                    query = "android"
                    selectedFields("id")
                    sortBy("rank", SortOrder.Asc)
                },
            ),
        )

        val searchJson = JSONObject(bridge.lastSearchJson ?: error("Expected native search request"))
        assertEquals("id", searchJson.getJSONArray("selectedFields").getString(0))
        assertEquals("rank", searchJson.getJSONObject("sort").getString("field"))
    }
}

private class FakeBridge(
    private val failingAddCalls: MutableSet<Int> = mutableSetOf(),
    private val searchHits: JSONArray = JSONArray()
        .put(
            JSONObject()
                .put("score", 1.0)
                .put(
                    "fields",
                    JSONObject()
                        .put(
                            "title",
                            JSONArray()
                                .put(JSONObject().put("type", "text").put("value", "result")),
                        ),
                ),
        ),
) : NativeBridge {
    var lastSearchQuery: String? = null
    var lastSearchJson: String? = null
    var addCalls: Int = 0
    var commitCalls: Int = 0
    var refreshCalls: Int = 0

    override fun openIndex(path: String, schemaJson: String, optionsJson: String): Long = 1

    override fun closeIndex(handle: Long) = Unit

    override fun addDocuments(handle: Long, documentsJson: String): String {
        addCalls += 1
        if (failingAddCalls.remove(addCalls)) {
            throw WriteException("Configured add failure")
        }
        val documents = JSONObject(documentsJson).getJSONArray("documents")
        return JSONObject().put("documentsAdded", documents.length()).toString()
    }

    override fun deleteTerm(handle: Long, field: String, valueJson: String): String =
        JSONObject().put("termsDeleted", 1).toString()

    override fun commit(handle: Long): String {
        commitCalls += 1
        return JSONObject().put("opstamp", 1L).toString()
    }

    override fun refresh(handle: Long): String {
        refreshCalls += 1
        return JSONObject().put("refreshed", true).toString()
    }

    override fun commitAndRefresh(handle: Long): String {
        commitCalls += 1
        refreshCalls += 1
        return JSONObject().put("opstamp", 1L).put("refreshed", true).toString()
    }

    override fun schemaInfo(handle: Long): String =
        JSONObject()
            .put(
                "fields",
                org.json.JSONArray()
                    .put(
                        JSONObject()
                            .put("name", "title")
                            .put("type", "text")
                            .put("stored", true)
                            .put("indexed", true)
                            .put("fast", false),
                    ),
            )
            .put("defaultSearchFields", org.json.JSONArray().put("title"))
            .toString()

    override fun search(handle: Long, queryJson: String): String {
        lastSearchJson = queryJson
        lastSearchQuery = JSONObject(queryJson).getString("query")
        return JSONObject()
            .put("totalHits", searchHits.length())
            .put("hits", searchHits)
            .toString()
    }
}
