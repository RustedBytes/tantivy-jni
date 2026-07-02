use serde_json::{Value as JsonValue, json};
use tantivy::collector::TopDocs;
use tantivy::index::Order;
use tantivy::query::QueryParser;
use tantivy::schema::{Field, TantivyDocument};

use crate::error::{NativeError, NativeResult};
use crate::{document, model, registry, validation};

pub(crate) fn search(handle: i64, query_json: &str) -> NativeResult<String> {
    let request: model::SearchRequest = serde_json::from_str(query_json)?;
    validate_search_request(&request)?;

    registry::with_index(handle, |index| {
        if request.reload_before_search {
            index
                .reader
                .reload()
                .map_err(|error| NativeError::Search(error.to_string()))?;
        }
        let search_fields = resolve_search_fields(index, &request)?;
        let hits = execute_search(index, &request, search_fields)?;
        Ok(json!({
            "totalHits": hits.len(),
            "hits": hits,
        })
        .to_string())
    })
}

fn resolve_search_fields(
    index: &model::NativeIndex,
    request: &model::SearchRequest,
) -> NativeResult<Vec<Field>> {
    let fields = if request.default_fields.is_empty() {
        index.default_search_fields.clone()
    } else {
        request
            .default_fields
            .iter()
            .map(|name| {
                index
                    .fields
                    .get(name)
                    .map(|field| field.field)
                    .ok_or_else(|| NativeError::Search(format!("unknown search field '{name}'")))
            })
            .collect::<NativeResult<Vec<_>>>()?
    };
    if fields.is_empty() {
        return Err(NativeError::Search(
            "at least one default search field is required".to_string(),
        ));
    }
    Ok(fields)
}

fn execute_search(
    index: &model::NativeIndex,
    request: &model::SearchRequest,
    search_fields: Vec<Field>,
) -> NativeResult<Vec<JsonValue>> {
    let query_parser = QueryParser::for_index(&index.index, search_fields);
    let query = query_parser
        .parse_query(&request.query)
        .map_err(|error| NativeError::Search(error.to_string()))?;
    let searcher = index.reader.searcher();
    let selected_fields = if request.selected_fields.is_empty() {
        None
    } else {
        validate_selected_fields(index, &request.selected_fields)?;
        Some(
            request
                .selected_fields
                .iter()
                .cloned()
                .collect::<std::collections::HashSet<_>>(),
        )
    };

    if let Some(sort) = &request.sort {
        return execute_sorted_search(
            index,
            request,
            sort,
            &searcher,
            query.as_ref(),
            selected_fields.as_ref(),
        );
    }

    let top_docs = searcher
        .search(&query, &top_docs(request).order_by_score())
        .map_err(|error| NativeError::Search(error.to_string()))?;

    let mut hits = Vec::with_capacity(top_docs.len());
    for (score, address) in top_docs {
        let document: TantivyDocument = searcher
            .doc(address)
            .map_err(|error| NativeError::Search(error.to_string()))?;
        hits.push(json!({
            "score": score,
            "fields": document::document_to_json(index, &document, selected_fields.as_ref()),
        }));
    }
    Ok(hits)
}

fn validate_selected_fields(
    index: &model::NativeIndex,
    selected_fields: &[String],
) -> NativeResult<()> {
    for name in selected_fields {
        let field = index
            .fields
            .get(name)
            .ok_or_else(|| NativeError::Search(format!("unknown selected field '{name}'")))?;
        if !field.stored {
            return Err(NativeError::Search(format!(
                "selected field '{name}' must be stored"
            )));
        }
    }
    Ok(())
}

fn execute_sorted_search(
    index: &model::NativeIndex,
    request: &model::SearchRequest,
    sort: &model::SortRequest,
    searcher: &tantivy::Searcher,
    query: &dyn tantivy::query::Query,
    selected_fields: Option<&std::collections::HashSet<String>>,
) -> NativeResult<Vec<JsonValue>> {
    let field = index
        .fields
        .get(&sort.field)
        .ok_or_else(|| NativeError::Search(format!("unknown sort field '{}'", sort.field)))?;
    if !field.fast {
        return Err(NativeError::Search(format!(
            "sort field '{}' must be a fast field",
            sort.field
        )));
    }

    match field.kind {
        model::FieldKind::I64 => sorted_hits(
            index,
            searcher.search(
                query,
                &top_docs(request).order_by_fast_field::<i64>(&sort.field, sort_order(sort.order)),
            ),
            selected_fields,
        ),
        model::FieldKind::U64 => sorted_hits(
            index,
            searcher.search(
                query,
                &top_docs(request).order_by_fast_field::<u64>(&sort.field, sort_order(sort.order)),
            ),
            selected_fields,
        ),
        model::FieldKind::F64 => sorted_hits(
            index,
            searcher.search(
                query,
                &top_docs(request).order_by_fast_field::<f64>(&sort.field, sort_order(sort.order)),
            ),
            selected_fields,
        ),
        _ => Err(NativeError::Search(format!(
            "sort field '{}' must be i64, u64, or f64",
            sort.field
        ))),
    }
}

fn sorted_hits<T>(
    index: &model::NativeIndex,
    result: tantivy::Result<Vec<(Option<T>, tantivy::DocAddress)>>,
    selected_fields: Option<&std::collections::HashSet<String>>,
) -> NativeResult<Vec<JsonValue>> {
    let sorted_docs = result.map_err(|error| NativeError::Search(error.to_string()))?;
    let searcher = index.reader.searcher();
    let mut hits = Vec::with_capacity(sorted_docs.len());
    for (_sort_value, address) in sorted_docs {
        let document: TantivyDocument = searcher
            .doc(address)
            .map_err(|error| NativeError::Search(error.to_string()))?;
        hits.push(json!({
            "score": 0.0,
            "fields": document::document_to_json(index, &document, selected_fields),
        }));
    }
    Ok(hits)
}

fn top_docs(request: &model::SearchRequest) -> TopDocs {
    TopDocs::with_limit(request.limit).and_offset(request.offset)
}

fn sort_order(order: model::SortOrder) -> Order {
    match order {
        model::SortOrder::Asc => Order::Asc,
        model::SortOrder::Desc => Order::Desc,
    }
}

fn validate_search_request(request: &model::SearchRequest) -> NativeResult<()> {
    if request.limit > validation::MAX_SEARCH_LIMIT {
        return Err(NativeError::Search(format!(
            "limit must be <= {}",
            validation::MAX_SEARCH_LIMIT
        )));
    }
    if request.offset > validation::MAX_SEARCH_OFFSET {
        return Err(NativeError::Search(format!(
            "offset must be <= {}",
            validation::MAX_SEARCH_OFFSET
        )));
    }
    Ok(())
}
