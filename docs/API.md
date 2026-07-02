# Tantivy Android Kotlin API

`tantivy-jni` exposes a Kotlin-first Android API backed by Rust and Tantivy through JNI. The public API is coroutine-native, UI-friendly, and typed; JSON stays internal to the JNI bridge.

Package:

```kotlin
import com.rustedbytes.tantivy.*
```

## API Layers

- `TantivyClient` is the recommended facade for opening indexes and building schemas, documents, and search requests.
- `TantivyIndex` is the safe high-level index handle for indexing, committing, refreshing, searching, observing state, and batch ingestion.
- `AdvancedTantivyIndex` is an opt-in advanced layer for native-backed features such as `commitAndRefresh`, schema inspection, and advanced search requests.

All blocking native calls run on the dispatcher configured in `IndexOptions`, which defaults to `Dispatchers.IO`.

## Opening An Index

Create a schema, then open an index in an app-owned directory such as `cacheDir` or `filesDir`.

```kotlin
val schema = TantivyClient.schema {
    string("id", stored = true, indexed = true)
    text("title", stored = true, indexed = true, defaultSearch = true)
    text("body", stored = true, indexed = true, defaultSearch = true)
    i64("publishedAt", stored = true, indexed = true, fast = true)
}

val index = TantivyClient.open(
    path = File(context.cacheDir, "articles-index").absolutePath,
    schema = schema,
    options = IndexOptions(
        create = true,
        writerThreads = 1,
        writerMemoryBytes = 50_000_000,
    ),
)
```

`TantivyClient.open(...)` is `suspend`, so call it from a coroutine such as `viewModelScope`, a repository scope, or a service scope.

## Schema Builder

Supported field types:

```kotlin
text("title")
string("id")
i64("createdAt", fast = true)
u64("count", fast = true)
f64("price", fast = true)
bool("published")
bytes("payload")
```

Common options:

```kotlin
val schema = TantivyClient.schema {
    text(
        name = "title",
        stored = true,
        indexed = true,
        defaultSearch = true,
        tokenizer = TokenizerMode.Default,
    )

    string(
        name = "sku",
        stored = true,
        indexed = true,
        tokenizer = TokenizerMode.Raw,
    )

    i64("rank", stored = true, indexed = true, fast = true)

    defaultSearchFields("title")
}
```

Use `fast = true` for numeric fields that you want to sort by. Default search fields must refer to fields declared in the schema.

## Documents

Build documents with typed helpers:

```kotlin
val document = TantivyClient.document {
    string("id", "article-1")
    text("title", "Tantivy on Android")
    text("body", "A searchable article body")
    i64("publishedAt", System.currentTimeMillis())
}

index.add(document)
```

For dynamic code, use `put`, `putAll`, and `repeated`:

```kotlin
val document = TantivyClient.document {
    put("id", FieldValue.StringValue("article-2"))
    repeated(
        "tag",
        listOf(
            FieldValue.StringValue("android"),
            FieldValue.StringValue("search"),
        ),
    )
}
```

## Mapping App Models

The library intentionally does not include reflection or annotation mapping in v1. Use `DocumentMapper<T>` when you want app-specific conversion.

```kotlin
data class Article(
    val id: String,
    val title: String,
    val publishedAt: Long,
)

val articleMapper = object : DocumentMapper<Article> {
    override fun toDocument(value: Article): IndexDocument = TantivyClient.document {
        string("id", value.id)
        text("title", value.title)
        i64("publishedAt", value.publishedAt)
    }

    override fun fromHit(hit: SearchHit): Article? {
        val id = (hit.fields["id"]?.firstOrNull() as? FieldValue.StringValue)?.value ?: return null
        val title = (hit.fields["title"]?.firstOrNull() as? FieldValue.Text)?.value ?: return null
        val publishedAt = (hit.fields["publishedAt"]?.firstOrNull() as? FieldValue.I64)?.value ?: return null
        return Article(id, title, publishedAt)
    }
}
```

## Writing, Committing, And Refreshing

Adding documents writes them to the index writer. Commit to persist changes, then refresh to make them searchable.

```kotlin
index.add(articleMapper.toDocument(article))
index.commit()
index.refresh()
```

Bulk add:

```kotlin
val result = index.addAll(articles.map(articleMapper::toDocument))
println("Added ${result.documentsAdded} documents")
```

Delete by indexed term:

```kotlin
index.delete("id", FieldValue.StringValue("article-1"))
index.commit()
index.refresh()
```

## Searching

Build a request with the facade:

```kotlin
val request = TantivyClient.query {
    query = "android search"
    defaultFields("title", "body")
    selectedFields("id", "title", "publishedAt")
    limit = 20
    offset = 0
    sortBy("publishedAt", SortOrder.Desc)
}

val page = index.search(request)
```

Read hits:

```kotlin
val articles = page.hits.mapNotNull(articleMapper::fromHit)
```

Notes:

- `selectedFields(...)` controls which stored fields are returned in hits.
- `sortBy(...)` is intended for supported fast fields, usually numeric fields declared with `fast = true`.
- `reloadBeforeSearch = true` asks native code to reload the reader before the search.
- `SearchQuery` remains available as a typealias for `SearchRequest`.

## UI-Friendly Search Flow

`searchFlow` is designed for UI query streams. It applies per-request flow options, cancels stale Kotlin work with `mapLatest`-style behavior, and emits loading, empty, success, or error states.

```kotlin
val uiStates: Flow<SearchState> = index.searchFlow(
    searchText.map { text ->
        TantivyClient.query {
            query = text
            defaultFields("title", "body")
            selectedFields("id", "title")
            limit = 20
            flowOptions = SearchFlowOptions(
                debounceMillis = 300,
                distinctUntilChanged = true,
                emitLoading = true,
            )
        }
    },
)
```

In a ViewModel:

```kotlin
val searchState: StateFlow<SearchState> = index.searchFlow(queryRequests)
    .stateIn(
        scope = viewModelScope,
        started = SharingStarted.WhileSubscribed(5_000),
        initialValue = SearchState.Empty,
    )
```

In Compose, collect with lifecycle-aware collection from your app layer:

```kotlin
val state by viewModel.searchState.collectAsStateWithLifecycle()
```

## Batch Indexing From Flow

Use `indexDocuments` for streaming ingestion. It emits progress between batches and checks coroutine cancellation before and after JNI calls.

```kotlin
index.indexDocuments(
    documents = articlesFlow.map(articleMapper::toDocument),
    options = BatchOptions(
        maxBatchSize = 500,
        commitPolicy = CommitPolicy.End,
        refreshPolicy = RefreshPolicy.AfterCommit,
        errorPolicy = BatchErrorPolicy.Stop,
        progressGranularity = ProgressGranularity.Batch,
    ),
).collect { progress ->
    when (progress) {
        is IndexingProgress.Batch -> {
            println("Indexed ${progress.totalIndexed}")
        }
        is IndexingProgress.Complete -> {
            println("Finished ${progress.totalIndexed}")
        }
    }
}
```

For long-running work owned by a caller scope:

```kotlin
val job = index.launchIndexing(
    scope = viewModelScope,
    documents = articlesFlow.map(articleMapper::toDocument),
    options = BatchOptions(maxBatchSize = 250),
)

job.cancel()
```

Cancellation is cooperative between native calls. If a JNI call has already started, it runs to completion; cancellation is checked before the next batch enters native code.

## Observing Index State

```kotlin
index.observeIndexState().collect { state ->
    when (state) {
        IndexState.Opening -> Unit
        IndexState.Open -> Unit
        IndexState.Committing -> Unit
        IndexState.Refreshing -> Unit
        IndexState.Closed -> Unit
        is IndexState.Error -> {
            val error = state.error
        }
    }
}
```

## Advanced APIs

Advanced APIs are opt-in because they expose native-specific behavior.

```kotlin
@OptIn(AdvancedTantivyApi::class)
suspend fun inspect(index: TantivyIndex) {
    val result = index.advanced.commitAndRefresh()
    val schemaInfo = index.advanced.schemaInfo()

    val page = index.advanced.nativeSearch(
        AdvancedSearchRequest(
            TantivyClient.query {
                query = "android"
                selectedFields("id", "title")
                sortBy("publishedAt", SortOrder.Desc)
            },
        ),
    )
}
```

Prefer the high-level `TantivyIndex` methods unless you specifically need advanced native behavior.

## Closing

Close indexes when the owning component is done with them.

```kotlin
index.closeSuspending()
```

`closeSuspending()` runs the native close on the configured dispatcher. `close()` is also available for `AutoCloseable`, but it is synchronous.

After close, public operations fail fast with `TantivyIndexClosedException`.

## Exceptions

Public errors derive from `TantivyException`:

```kotlin
try {
    val page = index.search(request)
} catch (error: TantivyException) {
    // Show an app-level error state or report it.
}
```

Known exception types:

- `SchemaException`
- `IndexOpenException`
- `WriteException`
- `SearchException`
- `NativeLibraryException`
- `TantivyIndexClosedException`

## Limits And Validation

Kotlin validates common limits before entering JNI:

- Search `limit` must be between `0` and `1000`.
- Search `offset` must be between `0` and `100000`.
- `BatchOptions.maxBatchSize` must be positive.
- `IndexOptions.writerThreads` must be between `1` and `8`.
- `IndexOptions.writerMemoryBytes` must be between `15000000` and `536870912`.
- `FieldValue.U64` must fit in a non-negative Kotlin `Long`.

Native code mirrors important validation and maps native failures into typed Kotlin exceptions where possible.

## Native Library

The Android wrapper loads the native library automatically:

```kotlin
System.loadLibrary("tantivy_jni")
```

If the native library is missing or cannot be loaded, opening or using the index fails with `NativeLibraryException`.

## Recommended Repository Pattern

Keep one owner for the index lifecycle, usually a repository or service. ViewModels should call suspend functions or collect flows, but they should not own native path setup unless the app architecture already puts persistence there.

```kotlin
class ArticleSearchRepository(
    private val index: TantivyIndex,
    private val mapper: DocumentMapper<Article>,
) {
    suspend fun upsert(article: Article) {
        index.add(mapper.toDocument(article))
        index.commit()
        index.refresh()
    }

    fun search(query: Flow<String>): Flow<SearchState> =
        index.searchFlow(
            query.map { text ->
                TantivyClient.query {
                    this.query = text
                    defaultFields("title", "body")
                    selectedFields("id", "title", "publishedAt")
                }
            },
        )
}
```
