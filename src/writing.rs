use serde_json::json;

use crate::document::{build_document, term_for_value};
use crate::model::{DeleteRequest, DocumentBatch, FieldValueRequest};
use crate::registry::with_index;
use crate::validation::MAX_DOCUMENTS_PER_BATCH;
use crate::{NativeError, NativeResult};

pub(crate) fn add_documents(handle: i64, documents_json: &str) -> NativeResult<String> {
    let batch: DocumentBatch = serde_json::from_str(documents_json)?;
    if batch.documents.len() > MAX_DOCUMENTS_PER_BATCH {
        return Err(NativeError::Write(format!(
            "batch exceeds {MAX_DOCUMENTS_PER_BATCH} documents"
        )));
    }

    let count = batch.documents.len();
    with_index(handle, |index| {
        for document in batch.documents {
            let document = build_document(index, document)?;
            index
                .writer
                .add_document(document)
                .map_err(|error| NativeError::Write(error.to_string()))?;
        }
        Ok(json!({ "documentsAdded": count }).to_string())
    })
}

pub(crate) fn delete_term(handle: i64, field_name: &str, value_json: &str) -> NativeResult<String> {
    let request: FieldValueRequest = serde_json::from_str::<DeleteRequest>(value_json)?.into();
    with_index(handle, |index| {
        let field = *index
            .fields
            .get(field_name)
            .ok_or_else(|| NativeError::Write(format!("unknown field '{field_name}'")))?;
        let term = term_for_value(field_name, field, request)?;
        index.writer.delete_term(term);
        Ok(json!({ "termsDeleted": 1 }).to_string())
    })
}

pub(crate) fn commit(handle: i64) -> NativeResult<String> {
    with_index(handle, |index| {
        let opstamp = index
            .writer
            .commit()
            .map_err(|error| NativeError::Write(error.to_string()))?;
        Ok(json!({ "opstamp": opstamp }).to_string())
    })
}

pub(crate) fn refresh(handle: i64) -> NativeResult<String> {
    with_index(handle, |index| {
        index
            .reader
            .reload()
            .map_err(|error| NativeError::Search(error.to_string()))?;
        Ok(json!({ "refreshed": true }).to_string())
    })
}

pub(crate) fn commit_and_refresh(handle: i64) -> NativeResult<String> {
    with_index(handle, |index| {
        let opstamp = index
            .writer
            .commit()
            .map_err(|error| NativeError::Write(error.to_string()))?;
        index
            .reader
            .reload()
            .map_err(|error| NativeError::Search(error.to_string()))?;
        Ok(json!({ "opstamp": opstamp, "refreshed": true }).to_string())
    })
}

pub(crate) fn schema_info(handle: i64) -> NativeResult<String> {
    with_index(handle, |index| {
        let fields = index
            .fields
            .iter()
            .map(|(name, field)| {
                json!({
                    "name": name,
                    "type": field.kind,
                    "stored": field.stored,
                    "indexed": field.indexed,
                    "fast": field.fast,
                    "experimental": field.experimental,
                })
            })
            .collect::<Vec<_>>();
        Ok(json!({
            "fields": fields,
            "defaultSearchFields": index.default_search_fields.iter()
                .filter_map(|field| index.field_names.get(field))
                .collect::<Vec<_>>(),
        })
        .to_string())
    })
}

impl From<DeleteRequest> for FieldValueRequest {
    fn from(value: DeleteRequest) -> Self {
        FieldValueRequest {
            kind: value.kind,
            value: value.value,
        }
    }
}
