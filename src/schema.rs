use std::collections::HashMap;

use tantivy::schema::{BytesOptions, Field, NumericOptions, STRING, Schema, TEXT, TextOptions};

use crate::model::{BuiltSchema, FieldInfo, FieldKind, FieldRequest, SchemaRequest, TokenizerMode};
use crate::{NativeError, NativeResult};

pub(crate) fn build_schema(request: &SchemaRequest) -> NativeResult<BuiltSchema> {
    if request.fields.is_empty() {
        return Err(NativeError::Schema(
            "at least one field is required".to_string(),
        ));
    }

    let mut builder = Schema::builder();
    let mut fields = HashMap::with_capacity(request.fields.len());
    let mut field_names = HashMap::with_capacity(request.fields.len());

    for field in &request.fields {
        validate_field_request(field, &fields)?;

        let tantivy_field = match field.kind {
            FieldKind::Text => builder.add_text_field(&field.name, text_options(TEXT, field)),
            FieldKind::String => builder.add_text_field(&field.name, text_options(STRING, field)),
            FieldKind::I64 => builder.add_i64_field(&field.name, numeric_options(field)),
            FieldKind::U64 => builder.add_u64_field(&field.name, numeric_options(field)),
            FieldKind::F64 => builder.add_f64_field(&field.name, numeric_options(field)),
            FieldKind::Bool => builder.add_bool_field(&field.name, numeric_options(field)),
            FieldKind::Bytes => builder.add_bytes_field(&field.name, bytes_options(field)),
        };

        fields.insert(
            field.name.clone(),
            FieldInfo {
                field: tantivy_field,
                kind: field.kind,
                stored: field.stored,
                indexed: field.indexed,
                fast: field.fast,
                experimental: field.experimental,
            },
        );
        field_names.insert(tantivy_field, field.name.clone());
    }

    let schema = builder.build();
    let default_search_fields = default_search_fields(request, &fields)?;

    Ok(BuiltSchema {
        schema,
        fields,
        field_names,
        default_search_fields,
    })
}

fn validate_field_request(
    field: &FieldRequest,
    fields: &HashMap<String, FieldInfo>,
) -> NativeResult<()> {
    if field.name.is_empty() {
        return Err(NativeError::Schema(
            "field name cannot be empty".to_string(),
        ));
    }
    if fields.contains_key(&field.name) {
        return Err(NativeError::Schema(format!(
            "duplicate field '{}'",
            field.name
        )));
    }
    Ok(())
}

fn default_search_fields(
    request: &SchemaRequest,
    fields: &HashMap<String, FieldInfo>,
) -> NativeResult<Vec<Field>> {
    if request.default_search_fields.is_empty() {
        return Ok(fields
            .values()
            .filter(|field| matches!(field.kind, FieldKind::Text | FieldKind::String))
            .map(|field| field.field)
            .collect());
    }

    request
        .default_search_fields
        .iter()
        .map(|name| {
            fields
                .get(name)
                .map(|field| field.field)
                .ok_or_else(|| NativeError::Schema(format!("unknown search field '{name}'")))
        })
        .collect()
}

fn text_options(base: TextOptions, field: &FieldRequest) -> TextOptions {
    let mut options = if field.indexed {
        match field.tokenizer {
            Some(TokenizerMode::Default) => TEXT,
            Some(TokenizerMode::Raw) => STRING,
            None => base,
        }
    } else {
        TextOptions::default()
    };
    if field.stored {
        options = options.set_stored();
    }
    options
}

fn numeric_options(field: &FieldRequest) -> NumericOptions {
    let mut options = NumericOptions::default();
    if field.indexed {
        options = options.set_indexed();
    }
    if field.stored {
        options = options.set_stored();
    }
    if field.fast {
        options = options.set_fast();
    }
    options
}

fn bytes_options(field: &FieldRequest) -> BytesOptions {
    let mut options = BytesOptions::default();
    if field.indexed {
        options = options.set_indexed();
    }
    if field.stored {
        options = options.set_stored();
    }
    if field.fast {
        options = options.set_fast();
    }
    options
}
