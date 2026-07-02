package com.rustedbytes.tantivy.fixture

import android.app.Activity
import android.os.Bundle
import android.widget.TextView
import com.rustedbytes.tantivy.CommitPolicy
import com.rustedbytes.tantivy.RefreshPolicy
import com.rustedbytes.tantivy.BatchOptions
import com.rustedbytes.tantivy.TantivyClient
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.asFlow
import kotlinx.coroutines.launch

class FixtureActivity : Activity() {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val status = TextView(this)
        setContentView(status)

        scope.launch {
            val index = TantivyClient.open(
                path = cacheDir.resolve("fixture-index").absolutePath,
                schema = TantivyClient.schema {
                    string("id")
                    text("title", defaultSearch = true)
                },
            )
            index.indexDocuments(
                listOf(
                    TantivyClient.document {
                        string("id", "fixture")
                        text("title", "maven consumer fixture")
                    },
                ).asFlow(),
                BatchOptions(
                    commitPolicy = CommitPolicy.End,
                    refreshPolicy = RefreshPolicy.AfterCommit,
                ),
            ).collect {}
            val page = index.search(TantivyClient.query { query = "fixture" })
            status.text = "hits=${page.hits.size}"
            index.closeSuspending()
        }
    }

    override fun onDestroy() {
        scope.cancel()
        super.onDestroy()
    }
}
