package com.rustedbytes.tantivy

import org.json.JSONArray

enum class FieldType(internal val wireName: String) {
    Text("text"),
    String("string"),
    I64("i64"),
    U64("u64"),
    F64("f64"),
    Bool("bool"),
    Bytes("bytes"),
    Date("date"),
    Json("json"),
    Facet("facet"),
    IpAddr("ipaddr"),
    ;

    companion object {
        internal fun fromWireName(wireName: String): FieldType =
            entries.firstOrNull { it.wireName == wireName }
                ?: throw NativeLibraryException("Unknown field type: $wireName")
    }
}

enum class TokenizerMode(internal val wireName: String) {
    Default("default"),
    Raw("raw"),
}

sealed class FieldValue {
    abstract val type: FieldType
    internal abstract fun rawJsonValue(): Any

    data class Text(val value: kotlin.String) : FieldValue() {
        override val type = FieldType.Text
        override fun rawJsonValue(): Any = value
    }

    data class StringValue(val value: kotlin.String) : FieldValue() {
        override val type = FieldType.String
        override fun rawJsonValue(): Any = value
    }

    data class I64(val value: Long) : FieldValue() {
        override val type = FieldType.I64
        override fun rawJsonValue(): Any = value
    }

    data class U64(val value: Long) : FieldValue() {
        init {
            require(value >= 0) { "U64 value must be non-negative in Kotlin Long representation" }
        }

        override val type = FieldType.U64
        override fun rawJsonValue(): Any = value
    }

    data class F64(val value: Double) : FieldValue() {
        override val type = FieldType.F64
        override fun rawJsonValue(): Any = value
    }

    data class Bool(val value: Boolean) : FieldValue() {
        override val type = FieldType.Bool
        override fun rawJsonValue(): Any = value
    }

    class Bytes(value: ByteArray) : FieldValue() {
        private val bytes: ByteArray = value.copyOf()

        override val type = FieldType.Bytes
        override fun rawJsonValue(): Any = JSONArray(bytes.map { it.toUByte().toInt() })

        fun toByteArray(): ByteArray = bytes.copyOf()

        override fun equals(other: Any?): Boolean {
            if (this === other) return true
            if (other !is Bytes) return false
            return bytes.contentEquals(other.bytes)
        }

        override fun hashCode(): Int {
            return bytes.contentHashCode()
        }
    }

    data class Date(val value: java.time.Instant) : FieldValue() {
        override val type = FieldType.Date
        override fun rawJsonValue(): Any = value.toEpochMilli()
    }

    data class Json(val value: org.json.JSONObject) : FieldValue() {
        override val type = FieldType.Json
        override fun rawJsonValue(): Any = value
    }

    /**
     * Represents a hierarchical facet path, e.g. "/electronics/phones".
     * Path segments are separated by '/' and must not be empty.
     */
    data class Facet(val path: kotlin.String) : FieldValue() {
        override val type = FieldType.Facet
        override fun rawJsonValue(): Any = path
    }

    /**
     * Represents an IP address (IPv4 or IPv6) as a string, e.g. "192.168.1.1" or "::1".
     * The value is stored and indexed as an IPv6 address internally (IPv4 is mapped to IPv6).
     */
    data class IpAddr(val address: kotlin.String) : FieldValue() {
        override val type = FieldType.IpAddr
        override fun rawJsonValue(): Any = address
    }
}

