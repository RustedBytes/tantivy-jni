package com.rustedbytes.tantivy

import org.json.JSONArray
import org.json.JSONObject

data class SchemaField(
    val name: String,
    val type: FieldType,
    val stored: Boolean = true,
    val indexed: Boolean = true,
    val fast: Boolean = false,
    val tokenizer: TokenizerMode? = null,
    val experimental: Boolean = false,
)

class IndexSchema private constructor(
    val fields: List<SchemaField>,
    val defaultSearchFields: List<String>,
) {
    init {
        require(fields.isNotEmpty()) { "At least one schema field is required" }
        require(fields.map { it.name }.toSet().size == fields.size) { "Schema field names must be unique" }
        require(defaultSearchFields.all { fieldName -> fields.any { it.name == fieldName } }) {
            "Default search fields must exist in schema"
        }
    }

    fun toJson(): String = JSONObject()
        .put(
            "fields",
            JSONArray(fields.map { field ->
                JSONObject()
                    .put("name", field.name)
                    .put("type", field.type.wireName)
                    .put("stored", field.stored)
                    .put("indexed", field.indexed)
                    .put("fast", field.fast)
                    .also { json ->
                        field.tokenizer?.let { json.put("tokenizer", it.wireName) }
                        if (field.experimental) json.put("experimental", true)
                    }
            }),
        )
        .put("defaultSearchFields", JSONArray(defaultSearchFields))
        .toString()

    companion object {
        fun build(block: Builder.() -> Unit): IndexSchema = Builder().apply(block).build()
    }

    class Builder {
        private val fields = mutableListOf<SchemaField>()
        private val defaultSearchFields = mutableListOf<String>()

        fun text(
            name: String,
            stored: Boolean = true,
            indexed: Boolean = true,
            defaultSearch: Boolean = true,
            tokenizer: TokenizerMode = TokenizerMode.Default,
        ) {
            field(name, FieldType.Text, stored, indexed, tokenizer = tokenizer)
            if (defaultSearch) defaultSearchFields += name
        }

        fun string(
            name: String,
            stored: Boolean = true,
            indexed: Boolean = true,
            defaultSearch: Boolean = false,
            tokenizer: TokenizerMode = TokenizerMode.Raw,
        ) {
            field(name, FieldType.String, stored, indexed, tokenizer = tokenizer)
            if (defaultSearch) defaultSearchFields += name
        }

        fun i64(name: String, stored: Boolean = true, indexed: Boolean = true, fast: Boolean = false) =
            field(name, FieldType.I64, stored, indexed, fast)

        fun u64(name: String, stored: Boolean = true, indexed: Boolean = true, fast: Boolean = false) =
            field(name, FieldType.U64, stored, indexed, fast)

        fun f64(name: String, stored: Boolean = true, indexed: Boolean = true, fast: Boolean = false) =
            field(name, FieldType.F64, stored, indexed, fast)

        fun bool(name: String, stored: Boolean = true, indexed: Boolean = true, fast: Boolean = false) =
            field(name, FieldType.Bool, stored, indexed, fast)

        fun bytes(name: String, stored: Boolean = true, indexed: Boolean = true, fast: Boolean = false) =
            field(name, FieldType.Bytes, stored, indexed, fast)

        fun field(
            name: String,
            type: FieldType,
            stored: Boolean = true,
            indexed: Boolean = true,
            fast: Boolean = false,
            tokenizer: TokenizerMode? = null,
            experimental: Boolean = false,
        ) {
            require(name.isNotBlank()) { "Field name cannot be blank" }
            fields += SchemaField(name, type, stored, indexed, fast, tokenizer, experimental)
        }

        fun defaultSearchFields(vararg names: String) {
            defaultSearchFields += names
        }

        fun build(): IndexSchema = IndexSchema(fields.toList(), defaultSearchFields.distinct())
    }
}
