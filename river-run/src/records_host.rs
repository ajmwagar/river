//! In-process `soil:records` host over `fungos-records`.
//!
//! This adapter is deliberately domain-free. It maps one book to the WIT
//! collection shape and scopes entity ids by collection. Devices, signals,
//! presence, and schemas live in `.river`.

use std::path::Path;

use anyhow::{Context, Result};
use fungos_records::{Record, Store};
use river_eval::sexp::Value;

use crate::Host;

pub struct SoilRecordsHost {
    store: Store,
    book: String,
}

impl SoilRecordsHost {
    pub fn open(root: impl AsRef<Path>, book: impl Into<String>) -> Result<Self> {
        let root = root.as_ref();
        let store = Store::open(root, &root.join("device_id"))
            .with_context(|| format!("open records at {}", root.display()))?;
        Ok(Self {
            store,
            book: book.into(),
        })
    }

    pub fn book(&self) -> &str {
        &self.book
    }

    fn scoped(collection: &str, id: &str) -> String {
        if collection.is_empty() {
            id.to_string()
        } else {
            format!("{collection}:{id}")
        }
    }

    fn unscoped(collection: &str, entity_id: &str) -> String {
        if collection.is_empty() {
            entity_id.to_string()
        } else {
            entity_id
                .strip_prefix(&format!("{collection}:"))
                .unwrap_or(entity_id)
                .to_string()
        }
    }

    fn entry_value(collection: &str, record: Record) -> Value {
        let body = serde_json::to_vec(&record.fields).unwrap_or_default();
        Value::List(vec![
            pair(
                "key",
                Value::List(vec![
                    pair("collection", Value::String(collection.to_string())),
                    pair(
                        "id",
                        Value::String(Self::unscoped(collection, &record.entity_id)),
                    ),
                ]),
            ),
            pair("schema", Value::String(record.schema_id)),
            pair("revision", Value::Int(record.updated_hlc as i64)),
            pair("updated-at-ms", Value::Int(record.updated_hlc as i64)),
            pair("origin", Value::String(record.origin)),
            pair("deleted", Value::Bool(record.deleted)),
            pair(
                "body",
                Value::List(body.iter().map(|byte| Value::Int(*byte as i64)).collect()),
            ),
        ])
    }

    fn scan_page(
        &self,
        collection: &str,
        cursor: Option<String>,
        limit: usize,
    ) -> std::result::Result<Value, String> {
        let cursor = cursor.map(|cursor| Self::scoped(collection, &cursor));
        let records = self
            .store
            .scan_records(&self.book, cursor.as_deref(), 4096)
            .map_err(|error| error.to_string())?;
        let prefix = format!("{collection}:");
        let entries: Vec<_> = records
            .into_iter()
            .filter(|record| collection.is_empty() || record.entity_id.starts_with(&prefix))
            .take(limit)
            .map(|record| Self::entry_value(collection, record))
            .collect();
        Ok(page_value(entries))
    }

    fn get_entry(&self, collection: &str, id: &str) -> std::result::Result<Option<Value>, String> {
        self.store
            .get_optional(&self.book, &Self::scoped(collection, id))
            .map(|record| record.map(|record| Self::entry_value(collection, record)))
            .map_err(|error| error.to_string())
    }
}

impl Host for SoilRecordsHost {
    fn granted(&self, interface: &str, function: &str) -> bool {
        interface == "soil:records"
            && matches!(
                function,
                "scan" | "get" | "upsert" | "delete" | "changes" | "scan-packed" | "cas"
            )
    }

    fn call(
        &mut self,
        interface: &str,
        function: &str,
        args: &[Value],
    ) -> std::result::Result<Value, String> {
        if interface != "soil:records" {
            return Err(format!("unknown interface `{interface}`"));
        }

        match function {
            "scan" | "changes" => {
                let collection = args
                    .first()
                    .map(value_text)
                    .ok_or_else(|| "scan needs a collection".to_string())?;
                let cursor = args.get(1).and_then(|value| match value {
                    Value::Nil => None,
                    value => Some(value_text(value)),
                });
                let limit = args.get(2).and_then(Value::as_f64).unwrap_or(50.0) as usize;
                Ok(ok(self.scan_page(&collection, cursor, limit)?))
            }
            "get" => {
                let key = args.first().ok_or_else(|| "get needs a key".to_string())?;
                let (collection, id) = parse_key(key)?;
                Ok(match self.get_entry(&collection, &id)? {
                    Some(entry) => ok(entry),
                    None => err("not-found"),
                })
            }
            "upsert" => {
                let entry = args
                    .first()
                    .cloned()
                    .ok_or_else(|| "upsert needs an entry".to_string())?;
                let key = field_value(&entry, "key");
                let (collection, id) = parse_key(&key)?;
                let schema = field_value(&entry, "schema")
                    .as_str()
                    .unwrap_or("river.record.v1")
                    .to_string();
                let fields = body_json(&field_value(&entry, "body"))?;
                self.store
                    .put_record(
                        &self.book,
                        &Self::scoped(&collection, &id),
                        &schema,
                        1,
                        fields,
                    )
                    .map_err(|error| error.to_string())?;
                Ok(match self.get_entry(&collection, &id)? {
                    Some(entry) => ok(entry),
                    None => err("io"),
                })
            }
            "delete" => {
                let key = args
                    .first()
                    .ok_or_else(|| "delete needs a key".to_string())?;
                let (collection, id) = parse_key(key)?;
                let Some(existing) = self.get_entry(&collection, &id)? else {
                    return Ok(err("not-found"));
                };
                let schema = field_value(&existing, "schema")
                    .as_str()
                    .unwrap_or("river.record.v1")
                    .to_string();
                self.store
                    .delete_record(&self.book, &Self::scoped(&collection, &id), &schema, 1)
                    .map_err(|error| error.to_string())?;
                Ok(ok(Value::Nil))
            }
            "scan-packed" | "cas" => Ok(err("unsupported")),
            other => Err(format!("unknown soil:records function `{other}`")),
        }
    }
}

fn parse_key(value: &Value) -> std::result::Result<(String, String), String> {
    let collection = field_value(value, "collection")
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| "key needs a collection".to_string())?;
    let id = field_value(value, "id")
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| "key needs an id".to_string())?;
    Ok((collection, id))
}

fn value_text(value: &Value) -> String {
    match value {
        Value::String(text) | Value::Symbol(text) | Value::Keyword(text) => text.clone(),
        other => other.to_string(),
    }
}

fn body_json(value: &Value) -> std::result::Result<serde_json::Value, String> {
    let bytes = match value {
        Value::List(items) => items
            .iter()
            .map(|item| match item {
                Value::Int(byte) if (0..=255).contains(byte) => Ok(*byte as u8),
                _ => Err("body must be a list of bytes".to_string()),
            })
            .collect::<std::result::Result<Vec<_>, _>>()?,
        Value::String(text) => text.as_bytes().to_vec(),
        _ => return Err("body must be a list of bytes or a string".to_string()),
    };
    serde_json::from_slice(&bytes).map_err(|error| format!("invalid JSON body: {error}"))
}

fn field_value(record: &Value, name: &str) -> Value {
    let Some(items) = record.as_list() else {
        return Value::Nil;
    };
    for item in items {
        let Some(pair) = item.as_list() else { continue };
        let field = pair
            .first()
            .and_then(|value| value.as_keyword().or_else(|| value.as_symbol()));
        if field == Some(name) {
            return pair.get(1).cloned().unwrap_or(Value::Nil);
        }
    }
    Value::Nil
}

fn pair(name: &str, value: Value) -> Value {
    Value::List(vec![Value::Keyword(name.to_string()), value])
}

fn page_value(entries: Vec<Value>) -> Value {
    Value::List(vec![
        pair("entries", Value::List(entries)),
        pair("next", Value::Nil),
    ])
}

fn ok(value: Value) -> Value {
    Value::List(vec![Value::Symbol("ok".to_string()), value])
}

fn err(code: &str) -> Value {
    Value::List(vec![
        Value::Symbol("err".to_string()),
        Value::Symbol(code.to_string()),
    ])
}
