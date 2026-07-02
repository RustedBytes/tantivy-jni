package com.rustedbytes.tantivy

import org.json.JSONArray
import org.json.JSONObject

internal fun documentsJson(documents: List<IndexDocument>): String =
    JSONObject()
        .put("documents", JSONArray(documents.map { it.toJsonObject() }))
        .toString()

internal fun deleteValueJson(value: FieldValue): String =
    JSONObject()
        .put("type", value.type.wireName)
        .put("value", value.rawJsonValue())
        .toString()

internal fun parseWriteResult(json: String): WriteResult {
    val objectJson = JSONObject(json)
    return WriteResult(objectJson.getInt("documentsAdded"))
}

internal fun parseDeleteResult(json: String): DeleteResult {
    val objectJson = JSONObject(json)
    return DeleteResult(objectJson.getInt("termsDeleted"))
}

internal fun parseCommitResult(json: String): CommitResult {
    val objectJson = JSONObject(json)
    return CommitResult(objectJson.getLong("opstamp"))
}

internal fun parseRefreshResult(json: String): RefreshResult {
    val objectJson = JSONObject(json)
    return RefreshResult(objectJson.getBoolean("refreshed"))
}

internal fun parseCommitRefreshResult(json: String): CommitRefreshResult {
    val objectJson = JSONObject(json)
    return CommitRefreshResult(
        opstamp = objectJson.getLong("opstamp"),
        refreshed = objectJson.getBoolean("refreshed"),
    )
}

internal fun parseSchemaInfo(json: String): SchemaInfo {
    val objectJson = JSONObject(json)
    val fieldsJson = objectJson.getJSONArray("fields")
    return SchemaInfo(
        fields = (0 until fieldsJson.length()).map { index ->
            val field = fieldsJson.getJSONObject(index)
            SchemaField(
                name = field.getString("name"),
                type = FieldType.fromWireName(field.getString("type")),
                stored = field.getBoolean("stored"),
                indexed = field.getBoolean("indexed"),
                fast = field.getBoolean("fast"),
                experimental = field.optBoolean("experimental", false),
            )
        },
        defaultSearchFields = objectJson.optJSONArray("defaultSearchFields")?.let { fields ->
            (0 until fields.length()).map { fields.getString(it) }
        }.orEmpty(),
    )
}

internal fun parseSearchPage(json: String): SearchPage {
    val objectJson = JSONObject(json)
    val hitsJson = objectJson.getJSONArray("hits")
    return SearchPage(
        totalHits = objectJson.getInt("totalHits"),
        hits = (0 until hitsJson.length()).map { index ->
            val hit = hitsJson.getJSONObject(index)
            val snippetsJson = hit.optJSONObject("snippets")
            val snippets = if (snippetsJson != null) {
                snippetsJson.keys().asSequence().associateWith { snippetsJson.getString(it) }
            } else {
                emptyMap()
            }
            SearchHit(
                score = hit.getDouble("score").toFloat(),
                fields = parseFields(hit.getJSONObject("fields")),
                snippets = snippets,
            )
        },
    )
}

internal fun parseDeleteAllResult(json: String): CommitResult {
    val objectJson = JSONObject(json)
    return CommitResult(objectJson.getLong("opstamp"))
}

private fun parseFields(fieldsJson: JSONObject): Map<String, List<FieldValue>> =
    fieldsJson.keys().asSequence().associateWith { name ->
        val values = fieldsJson.getJSONArray(name)
        (0 until values.length()).map { index -> parseFieldValue(values.getJSONObject(index)) }
    }

private fun parseFieldValue(valueJson: JSONObject): FieldValue =
    when (valueJson.getString("type")) {
        FieldType.Text.wireName -> FieldValue.Text(valueJson.getString("value"))
        FieldType.String.wireName -> FieldValue.StringValue(valueJson.getString("value"))
        FieldType.I64.wireName -> FieldValue.I64(valueJson.getLong("value"))
        FieldType.U64.wireName -> FieldValue.U64(valueJson.getLong("value"))
        FieldType.F64.wireName -> FieldValue.F64(valueJson.getDouble("value"))
        FieldType.Bool.wireName -> FieldValue.Bool(valueJson.getBoolean("value"))
        FieldType.Bytes.wireName -> FieldValue.Bytes(parseBytes(valueJson.getJSONArray("value")))
        FieldType.Date.wireName -> FieldValue.Date(java.time.Instant.ofEpochMilli(valueJson.getLong("value")))
        FieldType.Json.wireName -> FieldValue.Json(valueJson.getJSONObject("value"))
        FieldType.Facet.wireName -> FieldValue.Facet(valueJson.getString("value"))
        FieldType.IpAddr.wireName -> FieldValue.IpAddr(valueJson.getString("value"))
        else -> throw NativeLibraryException("Unknown field value type: ${valueJson.getString("type")}")
    }

private fun parseBytes(bytesJson: JSONArray): ByteArray =
    ByteArray(bytesJson.length()) { index -> bytesJson.getInt(index).toByte() }

