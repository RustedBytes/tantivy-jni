package com.rustedbytes.tantivy

import java.util.concurrent.atomic.AtomicLong
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.FlowPreview
import kotlinx.coroutines.Job
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.debounce
import kotlinx.coroutines.flow.flow
import kotlinx.coroutines.flow.transformLatest
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

@Suppress("SwallowedException", "TooGenericExceptionCaught")
class TantivyIndex internal constructor(
    handle: Long,
    private val options: IndexOptions,
    private val bridge: NativeBridge,
) : AutoCloseable {
    private val nativeHandle = AtomicLong(handle)
    private val state = MutableStateFlow<IndexState>(IndexState.Open)

    @AdvancedTantivyApi
    @OptIn(AdvancedTantivyApi::class)
    val advanced: AdvancedTantivyIndex = AdvancedTantivyIndex(this)

    fun observeIndexState(): StateFlow<IndexState> = state

    suspend fun add(document: IndexDocument): WriteResult = addAll(listOf(document))

    suspend fun addAll(documents: List<IndexDocument>): WriteResult {
        if (documents.isEmpty()) return WriteResult(0)
        return ioCall(stateOnError = true) { handle ->
            parseWriteResult(bridge.addDocuments(handle, documentsJson(documents)))
        }
    }

    suspend fun delete(field: String, value: FieldValue): DeleteResult =
        ioCall(stateOnError = true) { handle ->
            parseDeleteResult(bridge.deleteTerm(handle, field, deleteValueJson(value)))
        }

    suspend fun deleteQuery(query: String, defaultFields: List<String> = emptyList()): DeleteResult =
        ioCall(stateOnError = true) { handle ->
            parseDeleteResult(bridge.deleteQuery(handle, query, org.json.JSONArray(defaultFields).toString()))
        }

    suspend fun deleteAll(): CommitResult =
        ioCall(stateOnError = true) { handle ->
            parseDeleteAllResult(bridge.deleteAllDocuments(handle))
        }

    suspend fun commit(): CommitResult {
        state.value = IndexState.Committing
        return ioCall(stateOnError = true) { handle ->
            parseCommitResult(bridge.commit(handle))
        }.also {
            if (state.value !is IndexState.Error) state.value = IndexState.Open
        }
    }

    suspend fun refresh(): RefreshResult {
        state.value = IndexState.Refreshing
        return ioCall(stateOnError = true) { handle ->
            parseRefreshResult(bridge.refresh(handle))
        }.also {
            if (state.value !is IndexState.Error) state.value = IndexState.Open
        }
    }

    @AdvancedTantivyApi
    suspend fun commitAndRefresh(): CommitRefreshResult {
        state.value = IndexState.Committing
        return ioCall(stateOnError = true) { handle ->
            parseCommitRefreshResult(bridge.commitAndRefresh(handle))
        }.also {
            if (state.value !is IndexState.Error) state.value = IndexState.Open
        }
    }

    @AdvancedTantivyApi
    suspend fun schemaInfo(): SchemaInfo =
        ioCall(stateOnError = false) { handle ->
            parseSchemaInfo(bridge.schemaInfo(handle))
        }

    suspend fun search(query: SearchRequest): SearchPage =
        ioCall(stateOnError = false) { handle ->
            parseSearchPage(bridge.search(handle, query.toJson()))
        }

    @OptIn(FlowPreview::class, ExperimentalCoroutinesApi::class)
    fun searchFlow(query: Flow<SearchRequest>): Flow<SearchState> {
        val configuredQueries = flow {
            var previous: SearchRequest? = null
            query
                .debounce { request -> request.flowOptions.debounceMillis }
                .collect { request ->
                    if (!request.flowOptions.distinctUntilChanged || request != previous) {
                        emit(request)
                    }
                    previous = request
                }
            }
        return configuredQueries
            .transformLatest { searchQuery ->
                if (searchQuery.flowOptions.emitLoading) emit(SearchState.Loading)
                try {
                    val page = search(searchQuery)
                    emit(if (page.hits.isEmpty()) SearchState.Empty else SearchState.Success(page))
                } catch (error: CancellationException) {
                    throw error
                } catch (error: Throwable) {
                    emit(SearchState.Error(error.toTantivyException()))
                }
            }
    }

    fun indexDocuments(
        documents: Flow<IndexDocument>,
        options: BatchOptions = BatchOptions(),
    ): Flow<IndexingProgress> = flow {
        val batch = ArrayList<IndexDocument>(options.maxBatchSize)
        var total = 0

        suspend fun flush() {
            if (batch.isEmpty()) return
            currentCoroutineContext().ensureActive()
            val result = try {
                addAll(batch.toList())
            } catch (error: Throwable) {
                if (options.errorPolicy == BatchErrorPolicy.Continue) {
                    batch.clear()
                    return
                }
                throw error
            }
            total += result.documentsAdded
            if (options.commitPolicy == CommitPolicy.EveryBatch) commit()
            if (options.refreshPolicy == RefreshPolicy.AfterCommit && options.commitPolicy == CommitPolicy.EveryBatch) {
                refresh()
            }
            if (options.progressGranularity == ProgressGranularity.Batch) {
                emit(IndexingProgress.Batch(result.documentsAdded, total))
            }
            batch.clear()
            currentCoroutineContext().ensureActive()
        }

        documents.collect { document ->
            currentCoroutineContext().ensureActive()
            batch += document
            if (batch.size >= options.maxBatchSize) flush()
        }
        flush()
        if (options.commitPolicy == CommitPolicy.End) {
            commit()
            if (options.refreshPolicy == RefreshPolicy.AfterCommit) refresh()
        }
        if (options.refreshPolicy == RefreshPolicy.End) refresh()
        emit(IndexingProgress.Complete(total))
    }

    fun launchIndexing(
        scope: CoroutineScope,
        documents: Flow<IndexDocument>,
        options: BatchOptions = BatchOptions(),
    ): Job = scope.launch {
        indexDocuments(documents, options).collect {}
    }

    suspend fun closeSuspending() {
        val handle = nativeHandle.getAndSet(0)
        if (handle == 0L) return
        try {
            withContext(options.dispatcher) {
                bridge.closeIndex(handle)
            }
            state.value = IndexState.Closed
        } catch (error: CancellationException) {
            nativeHandle.compareAndSet(0, handle)
            throw error
        } catch (error: Throwable) {
            nativeHandle.compareAndSet(0, handle)
            throw error.toTantivyException()
        }
    }

    override fun close() {
        val handle = nativeHandle.getAndSet(0)
        if (handle == 0L) return
        try {
            bridge.closeIndex(handle)
            state.value = IndexState.Closed
        } catch (error: Throwable) {
            nativeHandle.compareAndSet(0, handle)
            throw error.toTantivyException()
        }
    }

    private suspend fun <T> ioCall(stateOnError: Boolean, block: (Long) -> T): T {
        currentCoroutineContext().ensureActive()
        val handle = requireHandle()
        return try {
            withContext(options.dispatcher) { block(handle) }
                .also { currentCoroutineContext().ensureActive() }
        } catch (error: CancellationException) {
            throw error
        } catch (error: Throwable) {
            val tantivyError = error.toTantivyException()
            if (stateOnError) state.value = IndexState.Error(tantivyError)
            throw tantivyError
        }
    }

    private fun requireHandle(): Long {
        val handle = nativeHandle.get()
        if (handle == 0L) throw TantivyIndexClosedException()
        return handle
    }

    companion object {
        suspend fun open(
            path: String,
            schema: IndexSchema,
            options: IndexOptions = IndexOptions(),
        ): TantivyIndex {
            return try {
                val handle = withContext(options.dispatcher) {
                    JniNativeBridge.openIndex(path, schema.toJson(), options.toJson())
                }
                if (handle <= 0L) throw IndexOpenException("native openIndex returned an invalid handle")
                TantivyIndex(handle, options, JniNativeBridge)
            } catch (error: CancellationException) {
                throw error
            } catch (error: Throwable) {
                throw error.toTantivyException()
            }
        }

        internal fun fromNativeHandle(
            handle: Long,
            options: IndexOptions,
            bridge: NativeBridge,
        ): TantivyIndex = TantivyIndex(handle, options, bridge)
    }
}

private fun Throwable.toTantivyException(): TantivyException =
    when (this) {
        is TantivyException -> this
        else -> NativeLibraryException(message ?: "Native Tantivy call failed", this)
    }
