package com.rustedbytes.tantivy

import androidx.test.platform.app.InstrumentationRegistry
import kotlin.test.Test
import kotlin.test.assertEquals
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
        }

        val index = TantivyIndex.open(indexDir.absolutePath, schema)
        index.indexDocuments(
            listOf(
                IndexDocument.build {
                    string("id", "1")
                    text("title", "android coroutine search")
                },
            ).asFlow(),
        ).collect {}
        index.commit()
        index.refresh()

        val page = index.search(SearchQuery("coroutine"))
        assertEquals(1, page.hits.size)
        index.closeSuspending()
    }
}
