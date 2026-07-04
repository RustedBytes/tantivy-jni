package com.rustedbytes.tantivy.sample

import android.app.Activity
import android.os.Bundle
import android.text.Editable
import android.text.TextWatcher
import android.view.ViewGroup
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.TextView
import com.rustedbytes.tantivy.BatchOptions
import com.rustedbytes.tantivy.CommitPolicy
import com.rustedbytes.tantivy.FieldValue
import com.rustedbytes.tantivy.IndexDocument
import com.rustedbytes.tantivy.ProgressGranularity
import com.rustedbytes.tantivy.RefreshPolicy
import com.rustedbytes.tantivy.SearchFlowOptions
import com.rustedbytes.tantivy.SearchHit
import com.rustedbytes.tantivy.SearchState
import com.rustedbytes.tantivy.SortOrder
import com.rustedbytes.tantivy.TantivyClient
import com.rustedbytes.tantivy.TantivyException
import com.rustedbytes.tantivy.TantivyIndex
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asFlow
import kotlinx.coroutines.launch

class MainActivity : Activity() {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)
    private val queryRequests = MutableStateFlow(searchRequest("android"))
    private var index: TantivyIndex? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val status = TextView(this)
        val queryInput = EditText(this)
        val results = TextView(this)

        queryInput.setText("android")
        queryInput.hint = "Search articles"
        queryInput.addTextChangedListener(
            object : TextWatcher {
                override fun beforeTextChanged(value: CharSequence?, start: Int, count: Int, after: Int) = Unit
                override fun onTextChanged(value: CharSequence?, start: Int, before: Int, count: Int) {
                    queryRequests.value = searchRequest(value?.toString().orEmpty())
                }

                override fun afterTextChanged(value: Editable?) = Unit
            },
        )

        setContentView(
            LinearLayout(this).apply {
                orientation = LinearLayout.VERTICAL
                setPadding(32, 32, 32, 32)
                addView(status, matchWidthWrapContent())
                addView(queryInput, matchWidthWrapContent())
                addView(results, matchWidthWrapContent())
            },
        )

        scope.launch {
            try {
                val opened = openAndSeedIndex(status)
                opened.searchFlow(queryRequests).collect { state ->
                    results.text = state.render()
                }
            } catch (error: TantivyException) {
                status.text = "Index error: ${error.message}"
            }
        }
    }

    override fun onDestroy() {
        scope.cancel()
        index?.close()
        super.onDestroy()
    }

    private suspend fun openAndSeedIndex(status: TextView): TantivyIndex {
        status.text = "Opening index"
        val opened = TantivyClient.open(
            path = cacheDir.resolve("sample-articles").absolutePath,
            schema = TantivyClient.schema {
                string("id")
                text("title", defaultSearch = true)
                text("body", defaultSearch = true)
                i64("rank", fast = true)
            },
        )
        // Track the index as soon as it exists so onDestroy always closes it,
        // even if seeding is cancelled; otherwise the native writer lock leaks
        // for the process lifetime and the next open fails.
        index = opened
        opened.indexDocuments(
            sampleArticles().map(::articleDocument).asFlow(),
            BatchOptions(
                maxBatchSize = 2,
                commitPolicy = CommitPolicy.End,
                refreshPolicy = RefreshPolicy.AfterCommit,
                progressGranularity = ProgressGranularity.CompletionOnly,
            ),
        ).collect { progress ->
            status.text = "Indexed $progress"
        }
        return opened
    }

    private fun articleDocument(article: Article): IndexDocument = TantivyClient.document {
        string("id", article.id)
        text("title", article.title)
        text("body", article.body)
        i64("rank", article.rank)
    }

    private fun SearchState.render(): String =
        when (this) {
            SearchState.Loading -> "Searching"
            SearchState.Empty -> "No matches"
            is SearchState.Error -> "Search error: ${error.message}"
            is SearchState.Success -> page.hits.joinToString(separator = "\n") { hit -> hit.render() }
        }

    private fun SearchHit.render(): String {
        val title = fields["title"]?.firstText().orEmpty()
        val id = fields["id"]?.firstText().orEmpty()
        return "$title\n$id"
    }

    private fun List<FieldValue>.firstText(): String? =
        when (val value = firstOrNull()) {
            is FieldValue.StringValue -> value.value
            is FieldValue.Text -> value.value
            else -> null
        }

    private fun searchRequest(query: String) = TantivyClient.query {
        this.query = query.ifBlank { "android" }
        selectedFields("id", "title")
        sortBy("rank", SortOrder.Desc)
        flowOptions = SearchFlowOptions(debounceMillis = 250, emitLoading = true)
    }

    private fun matchWidthWrapContent(): LinearLayout.LayoutParams =
        LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT)

    private fun sampleArticles(): List<Article> =
        listOf(
            Article("1", "Android coroutine search", "Index and search from UI-friendly flows.", 30),
            Article("2", "Rust Tantivy JNI", "Native Tantivy search exposed through a typed Kotlin API.", 20),
            Article("3", "Batch indexing", "Streaming documents into bounded native batches.", 10),
        )
}

private data class Article(
    val id: String,
    val title: String,
    val body: String,
    val rank: Long,
)
