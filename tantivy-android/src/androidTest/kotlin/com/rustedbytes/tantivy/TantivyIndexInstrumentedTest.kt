package com.rustedbytes.tantivy

import androidx.test.platform.app.InstrumentationRegistry
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.fail
import kotlinx.coroutines.flow.asFlow
import kotlinx.coroutines.test.runTest

class TantivyIndexInstrumentedTest {
    @Test
    fun indexSearchAndCloseInAppCache() = runTest {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val indexDir = context.cacheDir.resolve("tantivy-test-${System.nanoTime()}")
        val schema = IndexSchema.build {
            text("title")
            string("id")
            i64("rank", fast = true)
        }

        val index = TantivyIndex.open(indexDir.absolutePath, schema)
        index.indexDocuments(
            listOf(
                IndexDocument.build {
                    string("id", "1")
                    text("title", "android coroutine search")
                    i64("rank", 10)
                },
            ).asFlow(),
        ).collect {}
        index.commit()
        index.refresh()

        val page = index.search(
            TantivyClient.query {
                query = "coroutine"
                selectedFields("id")
                sortBy("rank", SortOrder.Desc)
            },
        )
        assertEquals(1, page.hits.size)
        assertEquals(setOf("id"), page.hits.single().fields.keys)
        index.closeSuspending()
        try {
            index.search(SearchQuery("coroutine"))
            fail("Expected closed index to reject search")
        } catch (_: TantivyIndexClosedException) {
            // Expected path.
        }
    }
}
