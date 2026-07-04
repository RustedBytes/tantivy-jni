package com.rustedbytes.tantivy

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue
import kotlinx.coroutines.awaitCancellation
import kotlinx.coroutines.async
import kotlinx.coroutines.Dispatchers
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
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

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
    fun closeWhileAddIsInFlightClosesHandleAndRejectsLaterOperations() = runTest {
        val enteredAdd = CountDownLatch(1)
        val releaseAdd = CountDownLatch(1)
        val bridge = FakeBridge(
            beforeAdd = {
                enteredAdd.countDown()
                check(releaseAdd.await(5, TimeUnit.SECONDS))
            },
        )
        val index = TantivyIndex.fromNativeHandle(1, IndexOptions(dispatcher = Dispatchers.Default), bridge)

        val addJob = async(Dispatchers.Default) {
            index.add(IndexDocument.build { text("title", "value") })
        }

        assertTrue(enteredAdd.await(5, TimeUnit.SECONDS))
        index.closeSuspending()
        assertFailsWith<TantivyIndexClosedException> {
            index.search(SearchQuery("value"))
        }
        releaseAdd.countDown()

        assertEquals(1, addJob.await().documentsAdded)
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

        assertEquals(listOf<SearchState>(SearchState.Empty), states)
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

    @Test
    fun allFieldTypesAndSchemaMethodsAreExercised() {
        // This exercises all schema builder methods
        val schema = TantivyClient.schema {
            text("text_field")
            string("string_field")
            u64("u64_field", stored = true, indexed = true, fast = true)
            i64("i64_field", stored = true, indexed = true, fast = true)
            f64("f64_field", stored = true, indexed = true, fast = true)
            date("date_field", stored = true, indexed = true, fast = true)
            facet("facet_field")
            bytes("bytes_field", stored = true, indexed = true, fast = true)
            ipAddr("ip_addr_field", stored = true, indexed = true, fast = true)
            json("json_field")
            bool("bool_field")
            defaultSearchFields("text_field")
        }

        // This exercises all FieldValue types in document builder
        val document = TantivyClient.document {
            text("text_field", "value")
            string("string_field", "value")
            u64("u64_field", 1L)
            i64("i64_field", 1L)
            f64("f64_field", 1.0)
            date("date_field", java.time.Instant.ofEpochMilli(1000))
            facet("facet_field", "/category/test")
            bytes("bytes_field", byteArrayOf(1, 2, 3))
            ipAddr("ip_addr_field", "127.0.0.1")
            json("json_field", JSONObject("""{"key":"value"}"""))
            bool("bool_field", true)
        }

        // Verify some properties to ensure they compiled into the document
        assertEquals(FieldValue.U64(1L), document.fields["u64_field"]?.first())
        assertEquals(FieldValue.I64(1L), document.fields["i64_field"]?.first())
        assertEquals(FieldValue.F64(1.0), document.fields["f64_field"]?.first())
        assertEquals(FieldValue.Date(java.time.Instant.ofEpochMilli(1000)), document.fields["date_field"]?.first())
        assertEquals(FieldValue.Facet("/category/test"), document.fields["facet_field"]?.first())
        assertEquals(FieldValue.IpAddr("127.0.0.1"), document.fields["ip_addr_field"]?.first())
        val expectedJson = FieldValue.Json(JSONObject("""{"key":"value"}"""))
        val actualJson = document.fields["json_field"]?.first()
        assertEquals(expectedJson.rawJsonValue().toString(), actualJson?.rawJsonValue().toString())
        val bytesVal = document.fields["bytes_field"]?.first() as FieldValue.Bytes
        assertTrue(bytesVal.toByteArray().contentEquals(byteArrayOf(1, 2, 3)))
    }
    @Test
    fun indexOptionsDefaultsAreCorrect() {
        val options = IndexOptions()
        assertEquals(1, options.writerThreads)
        assertEquals(50_000_000, options.writerMemoryBytes)
    }

    @Test
    fun tantivyIndexOpenFailsWithoutNativeLibrary() = runTest {
        val failure = assertFailsWith<Throwable> {
            TantivyClient.open("test_path", TantivyClient.schema { text("title") })
        }

        assertTrue(
            failure is NativeLibraryException || failure is UnsatisfiedLinkError,
            "Expected native library load failure but was ${failure::class.qualifiedName}",
        )
    }

    @Test
    fun fieldValueEqualsAndHashCode() {
        val b1 = FieldValue.Bytes(byteArrayOf(1, 2))
        val b2 = FieldValue.Bytes(byteArrayOf(1, 2))
        val b3 = FieldValue.Bytes(byteArrayOf(1, 3))
        assertEquals(b1, b2)
        assertEquals(b1.hashCode(), b2.hashCode())
        assertTrue(b1 != b3)

        assertEquals(FieldValue.U64(1L), FieldValue.U64(1L))
        assertEquals(FieldValue.U64(1L).hashCode(), FieldValue.U64(1L).hashCode())

        assertEquals(FieldValue.I64(1L), FieldValue.I64(1L))
        assertEquals(FieldValue.F64(1.0), FieldValue.F64(1.0))
        assertEquals(FieldValue.Bool(true), FieldValue.Bool(true))
    }
}
private class FakeBridge(
    private val failingAddCalls: MutableSet<Int> = mutableSetOf(),
    private val beforeAdd: () -> Unit = {},
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
        beforeAdd()
        if (failingAddCalls.remove(addCalls)) {
            throw WriteException("Configured add failure")
        }
        val documents = JSONObject(documentsJson).getJSONArray("documents")
        return JSONObject().put("documentsAdded", documents.length()).toString()
    }

    override fun deleteTerm(handle: Long, field: String, valueJson: String): String =
        JSONObject().put("termsDeleted", 1).toString()

    override fun deleteQuery(handle: Long, query: String, defaultFieldsJson: String): String =
        JSONObject().put("termsDeleted", 1).toString()

    override fun deleteAllDocuments(handle: Long): String =
        JSONObject().put("opstamp", 1L).toString()

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
