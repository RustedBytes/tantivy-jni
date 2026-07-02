package com.rustedbytes.tantivy

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.asFlow
import kotlinx.coroutines.flow.toList
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.json.JSONObject

@OptIn(ExperimentalCoroutinesApi::class)
class TantivyIndexTest {
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
    fun searchFlowUsesLatestQuery() = runTest {
        val dispatcher = StandardTestDispatcher(testScheduler)
        val bridge = FakeBridge()
        val index = TantivyIndex.fromNativeHandle(1, IndexOptions(dispatcher = dispatcher), bridge)
        val queries = MutableSharedFlow<SearchQuery>(extraBufferCapacity = 2)
        val states = mutableListOf<SearchState>()

        val job = launch {
            index.searchFlow(queries).toList(states)
        }

        queries.emit(SearchQuery("old"))
        queries.emit(SearchQuery("new"))
        advanceTimeBy(300)
        advanceUntilIdle()

        assertEquals("new", bridge.lastSearchQuery)
        job.cancel()
    }

    @OptIn(AdvancedTantivyApi::class)
    @Test
    fun advancedApiCallsBridgeMethods() = runTest {
        val bridge = FakeBridge()
        val index = TantivyIndex.fromNativeHandle(1, IndexOptions(dispatcher = UnconfinedTestDispatcher(testScheduler)), bridge)

        assertEquals(true, index.advanced.commitAndRefresh().refreshed)
        assertEquals("title", index.advanced.schemaInfo().fields.first().name)
    }
}

private class FakeBridge : NativeBridge {
    var lastSearchQuery: String? = null

    override fun openIndex(path: String, schemaJson: String, optionsJson: String): Long = 1

    override fun closeIndex(handle: Long) = Unit

    override fun addDocuments(handle: Long, documentsJson: String): String {
        val documents = JSONObject(documentsJson).getJSONArray("documents")
        return JSONObject().put("documentsAdded", documents.length()).toString()
    }

    override fun deleteTerm(handle: Long, field: String, valueJson: String): String =
        JSONObject().put("termsDeleted", 1).toString()

    override fun commit(handle: Long): String =
        JSONObject().put("opstamp", 1L).toString()

    override fun refresh(handle: Long): String =
        JSONObject().put("refreshed", true).toString()

    override fun commitAndRefresh(handle: Long): String =
        JSONObject().put("opstamp", 1L).put("refreshed", true).toString()

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
        lastSearchQuery = JSONObject(queryJson).getString("query")
        return JSONObject()
            .put("totalHits", 1)
            .put(
                "hits",
                org.json.JSONArray()
                    .put(
                        JSONObject()
                            .put("score", 1.0)
                            .put(
                                "fields",
                                JSONObject()
                                    .put(
                                        "title",
                                        org.json.JSONArray()
                                            .put(JSONObject().put("type", "text").put("value", lastSearchQuery)),
                                    ),
                            ),
                    ),
            )
            .toString()
    }
}
