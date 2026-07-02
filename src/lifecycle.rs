use std::path::Path;

use tantivy::{Index, ReloadPolicy};

use crate::error::{NativeError, NativeResult};
use crate::{model, registry, schema, validation};

pub(crate) fn open_index(path: &str, schema_json: &str, options_json: &str) -> NativeResult<i64> {
    let schema_request: model::SchemaRequest = serde_json::from_str(schema_json)?;
    let options: model::OpenOptions = serde_json::from_str(options_json)?;
    validate_open_options(&options)?;

    let built_schema = schema::build_schema(&schema_request)?;
    let path = Path::new(path);
    std::fs::create_dir_all(path).map_err(|error| NativeError::Open(error.to_string()))?;

    let index = open_or_create_tantivy_index(path, options.create, &built_schema.schema)?;
    let writer = index
        .writer_with_num_threads(options.writer_threads, options.writer_memory_bytes)
        .map_err(|error| NativeError::Open(error.to_string()))?;
    let reader = index
        .reader_builder()
        .reload_policy(ReloadPolicy::Manual)
        .try_into()
        .map_err(|error| NativeError::Open(error.to_string()))?;

    registry::insert_index(model::NativeIndex {
        index,
        fields: built_schema.fields,
        field_names: built_schema.field_names,
        default_search_fields: built_schema.default_search_fields,
        writer,
        reader,
    })
}

pub(crate) fn close_index(handle: i64) -> NativeResult<()> {
    registry::remove_index(handle)
}

fn open_or_create_tantivy_index(
    path: &Path,
    create: bool,
    schema: &tantivy::schema::Schema,
) -> NativeResult<Index> {
    if path.join("meta.json").exists() {
        return Index::open_in_dir(path).map_err(|error| NativeError::Open(error.to_string()));
    }
    if create {
        return Index::create_in_dir(path, schema.clone())
            .map_err(|error| NativeError::Open(error.to_string()));
    }
    Err(NativeError::Open(format!(
        "index does not exist at '{}'",
        path.display()
    )))
}

fn validate_open_options(options: &model::OpenOptions) -> NativeResult<()> {
    if !(1..=validation::MAX_WRITER_THREADS).contains(&options.writer_threads) {
        return Err(NativeError::Open(format!(
            "writerThreads must be between 1 and {}",
            validation::MAX_WRITER_THREADS
        )));
    }
    if !(validation::MIN_WRITER_MEMORY_BYTES..=validation::MAX_WRITER_MEMORY_BYTES)
        .contains(&options.writer_memory_bytes)
    {
        return Err(NativeError::Open(format!(
            "writerMemoryBytes must be between {} and {}",
            validation::MIN_WRITER_MEMORY_BYTES,
            validation::MAX_WRITER_MEMORY_BYTES
        )));
    }
    Ok(())
}
