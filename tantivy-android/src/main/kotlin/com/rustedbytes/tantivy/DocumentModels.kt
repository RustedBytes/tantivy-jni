package com.rustedbytes.tantivy

import org.json.JSONArray
import org.json.JSONObject

private const val MAX_FIELD_VALUES_PER_DOCUMENT = 10_000

data class IndexDocument(val fields: Map<String, List<FieldValue>>) {
    init {
        val valueCount = fields.values.sumOf { it.size }
        require(valueCount <= MAX_FIELD_VALUES_PER_DOCUMENT) {
            "Document cannot contain more than $MAX_FIELD_VALUES_PER_DOCUMENT field values"
        }
    }

    fun toJsonObject(): JSONObject = JSONObject()
        .put(
            "fields",
            JSONObject(fields.mapValues { (_, values) ->
                JSONArray(values.map { value ->
                    JSONObject()
                        .put("type", value.type.wireName)
                        .put("value", value.rawJsonValue())
                })
            }),
        )

    companion object {
        fun build(block: Builder.() -> Unit): IndexDocument = Builder().apply(block).build()
    }

    class Builder {
        private val fields = linkedMapOf<String, MutableList<FieldValue>>()

        fun field(name: String, value: FieldValue) {
            require(name.isNotBlank()) { "Field name cannot be blank" }
            fields.getOrPut(name) { mutableListOf() } += value
        }

        fun text(name: String, value: String) = field(name, FieldValue.Text(value))
        fun string(name: String, value: String) = field(name, FieldValue.StringValue(value))
        fun i64(name: String, value: Long) = field(name, FieldValue.I64(value))
        fun u64(name: String, value: Long) = field(name, FieldValue.U64(value))
        fun f64(name: String, value: Double) = field(name, FieldValue.F64(value))
        fun bool(name: String, value: Boolean) = field(name, FieldValue.Bool(value))
        fun bytes(name: String, value: ByteArray) = field(name, FieldValue.Bytes(value))
        fun date(name: String, value: java.time.Instant) = field(name, FieldValue.Date(value))
        fun json(name: String, value: org.json.JSONObject) = field(name, FieldValue.Json(value))
        fun put(name: String, value: FieldValue) = field(name, value)
        fun putAll(values: Map<String, FieldValue>) = values.forEach { (name, value) -> field(name, value) }
        fun repeated(name: String, values: Iterable<FieldValue>) = values.forEach { field(name, it) }

        fun build(): IndexDocument = IndexDocument(fields.mapValues { it.value.toList() })
    }
}
