use serde_json::{Value as JsonValue, json};
use tantivy::schema::{TantivyDocument, Term, Value};

use crate::model::{DocumentRequest, FieldInfo, FieldKind, FieldValueRequest, NativeIndex};
use crate::validation::{MAX_FIELD_VALUES_PER_DOCUMENT, MAX_STORED_BYTES};
use crate::{NativeError, NativeResult};

pub(crate) fn build_document(
    index: &NativeIndex,
    request: DocumentRequest,
) -> NativeResult<TantivyDocument> {
    let mut document = TantivyDocument::default();
    let mut value_count = 0usize;

    for (name, values) in request.fields {
        value_count = value_count.saturating_add(values.len());
        if value_count > MAX_FIELD_VALUES_PER_DOCUMENT {
            return Err(NativeError::Write(format!(
                "document exceeds {MAX_FIELD_VALUES_PER_DOCUMENT} field values"
            )));
        }

        let field = *index
            .fields
            .get(&name)
            .ok_or_else(|| NativeError::Write(format!("unknown field '{name}'")))?;
        for value in values {
            ensure_kind(&name, field, value.kind)?;
            add_value(&mut document, field, value)?;
        }
    }
    Ok(document)
}

pub(crate) fn term_for_value(
    field_name: &str,
    field: FieldInfo,
    value: FieldValueRequest,
) -> NativeResult<Term> {
    ensure_kind(field_name, field, value.kind)?;
    match field.kind {
        FieldKind::Text | FieldKind::String => {
            let text = json_string(value.value, "text value")?;
            Ok(Term::from_field_text(field.field, &text))
        }
        FieldKind::I64 => Ok(Term::from_field_i64(
            field.field,
            json_i64(value.value, "i64 value")?,
        )),
        FieldKind::U64 => Ok(Term::from_field_u64(
            field.field,
            json_u64(value.value, "u64 value")?,
        )),
        FieldKind::F64 => Ok(Term::from_field_f64(
            field.field,
            json_f64(value.value, "f64 value")?,
        )),
        FieldKind::Bool => Ok(Term::from_field_bool(
            field.field,
            json_bool(value.value, "bool value")?,
        )),
        FieldKind::Bytes => Ok(Term::from_field_bytes(
            field.field,
            &json_bytes(value.value)?,
        )),
    }
}

pub(crate) fn document_to_json(
    index: &NativeIndex,
    document: &TantivyDocument,
    selected_fields: Option<&std::collections::HashSet<String>>,
) -> JsonValue {
    let mut fields = serde_json::Map::new();
    for (field, value) in document.field_values() {
        let Some(name) = index.field_names.get(&field) else {
            continue;
        };
        if selected_fields.is_some_and(|selected| !selected.contains(name)) {
            continue;
        }
        let Some(info) = index.fields.get(name) else {
            continue;
        };
        let Some(value) = stored_value_to_json(info.kind, &value) else {
            continue;
        };
        let values = fields
            .entry(name.clone())
            .or_insert_with(|| JsonValue::Array(Vec::new()));
        if let Some(values) = values.as_array_mut() {
            values.push(value);
        }
    }
    JsonValue::Object(fields)
}

fn add_value(
    document: &mut TantivyDocument,
    field: FieldInfo,
    value: FieldValueRequest,
) -> NativeResult<()> {
    match field.kind {
        FieldKind::Text | FieldKind::String => {
            document.add_text(field.field, json_string(value.value, "text value")?);
        }
        FieldKind::I64 => document.add_i64(field.field, json_i64(value.value, "i64 value")?),
        FieldKind::U64 => document.add_u64(field.field, json_u64(value.value, "u64 value")?),
        FieldKind::F64 => document.add_f64(field.field, json_f64(value.value, "f64 value")?),
        FieldKind::Bool => document.add_bool(field.field, json_bool(value.value, "bool value")?),
        FieldKind::Bytes => document.add_bytes(field.field, &json_bytes(value.value)?),
    }
    Ok(())
}

fn ensure_kind(field_name: &str, field: FieldInfo, actual: FieldKind) -> NativeResult<()> {
    if actual != field.kind {
        return Err(NativeError::Write(format!(
            "field '{field_name}' expected {:?}, got {:?}",
            field.kind, actual
        )));
    }
    Ok(())
}

fn stored_value_to_json<'a, V>(kind: FieldKind, value: &V) -> Option<JsonValue>
where
    V: Value<'a>,
{
    match kind {
        FieldKind::Text => value
            .as_str()
            .map(|value| json!({ "type": "text", "value": value })),
        FieldKind::String => value
            .as_str()
            .map(|value| json!({ "type": "string", "value": value })),
        FieldKind::I64 => value
            .as_i64()
            .map(|value| json!({ "type": "i64", "value": value })),
        FieldKind::U64 => value
            .as_u64()
            .map(|value| json!({ "type": "u64", "value": value })),
        FieldKind::F64 => value
            .as_f64()
            .map(|value| json!({ "type": "f64", "value": value })),
        FieldKind::Bool => value
            .as_bool()
            .map(|value| json!({ "type": "bool", "value": value })),
        FieldKind::Bytes => value.as_bytes().map(|bytes| {
            json!({
                "type": "bytes",
                "value": bytes.iter().map(|byte| *byte as u64).collect::<Vec<_>>()
            })
        }),
    }
}

fn json_string(value: JsonValue, expected: &str) -> NativeResult<String> {
    value
        .as_str()
        .map(ToString::to_string)
        .ok_or_else(|| NativeError::Write(format!("expected {expected}")))
}

fn json_i64(value: JsonValue, expected: &str) -> NativeResult<i64> {
    value
        .as_i64()
        .ok_or_else(|| NativeError::Write(format!("expected {expected}")))
}

fn json_u64(value: JsonValue, expected: &str) -> NativeResult<u64> {
    value
        .as_u64()
        .ok_or_else(|| NativeError::Write(format!("expected {expected}")))
}

fn json_f64(value: JsonValue, expected: &str) -> NativeResult<f64> {
    value
        .as_f64()
        .ok_or_else(|| NativeError::Write(format!("expected {expected}")))
}

fn json_bool(value: JsonValue, expected: &str) -> NativeResult<bool> {
    value
        .as_bool()
        .ok_or_else(|| NativeError::Write(format!("expected {expected}")))
}

fn json_bytes(value: JsonValue) -> NativeResult<Vec<u8>> {
    let bytes = value
        .as_array()
        .ok_or_else(|| NativeError::Write("expected bytes array".to_string()))?;
    if bytes.len() > MAX_STORED_BYTES {
        return Err(NativeError::Write(format!(
            "bytes value exceeds {MAX_STORED_BYTES} bytes"
        )));
    }

    bytes
        .iter()
        .map(|value| {
            let byte = value
                .as_u64()
                .ok_or_else(|| NativeError::Write("expected byte value".to_string()))?;
            u8::try_from(byte).map_err(|_| NativeError::Write("byte out of range".to_string()))
        })
        .collect()
}
