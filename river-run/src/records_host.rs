//! In-process `soil:records` host over `fungos-records`.
//!
//! This is the real host boundary: `host-call "soil:records" "scan"` calls the
//! same store the Soil dev host uses. There is no HTTP bridge in this path.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use fungos_records::{Record, Store};
use river_eval::sexp::Value;

use crate::Host;

#[derive(Clone, Debug)]
struct Namespace {
    name: String,
    book: String,
    collection: String,
    read: bool,
    write: bool,
}

impl Namespace {
    fn write(name: &str, book: &str, collection: &str) -> Self {
        Self {
            name: name.to_string(),
            book: book.to_string(),
            collection: collection.to_string(),
            read: true,
            write: true,
        }
    }
}

pub struct SoilRecordsHost {
    store: Store,
    namespaces: BTreeMap<String, Namespace>,
}

impl SoilRecordsHost {
    /// Open one records root with the Instinct namespaces.
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref();
        let store = Store::open(root, &root.join("device_id"))
            .with_context(|| format!("open records at {}", root.display()))?;
        let namespaces = [
            Namespace::write(
                "signal:avery",
                "dev/personal/avery/telemetry",
                "signal:avery",
            ),
            Namespace::write(
                "device:avery",
                "dev/personal/avery/telemetry",
                "device:avery",
            ),
            Namespace::write(
                "presence:avery",
                "dev/personal/avery/telemetry",
                "presence:avery",
            ),
        ]
        .into_iter()
        .map(|namespace| (namespace.name.clone(), namespace))
        .collect();

        Ok(Self { store, namespaces })
    }

    /// Seed the first Instinct fixture: one watch and two signals.
    pub fn seed_instinct(&mut self) -> Result<()> {
        self.seed_device(
            "device:avery",
            "pine:1",
            serde_json::json!({
                "kind": "wearable.watch",
                "model": "PineTime",
                "charging": false,
                "link": "connected"
            }),
        )?;
        self.seed_signal(
            "signal:avery",
            "pine:1:hr",
            serde_json::json!({
                "kind": "body.hr.v1",
                "value": { "bpm": 61, "contact": "good" },
                "wearer": "avery"
            }),
        )?;
        self.seed_signal(
            "signal:avery",
            "pine:1:motion",
            serde_json::json!({
                "kind": "body.motion.v1",
                "value": { "moving": true },
                "wearer": "avery"
            }),
        )?;
        Ok(())
    }

    fn seed_device(&mut self, collection: &str, id: &str, fields: serde_json::Value) -> Result<()> {
        let namespace = self
            .namespaces
            .get(collection)
            .ok_or_else(|| anyhow::anyhow!("unknown collection `{collection}`"))?;
        self.store.put_record(
            &namespace.book,
            &Self::scoped(namespace, id),
            "fungos.device/state.v1",
            1,
            fields,
        )?;
        Ok(())
    }

    fn seed_signal(&mut self, collection: &str, id: &str, fields: serde_json::Value) -> Result<()> {
        let namespace = self
            .namespaces
            .get(collection)
            .ok_or_else(|| anyhow::anyhow!("unknown collection `{collection}`"))?;
        self.store.put_record(
            &namespace.book,
            &Self::scoped(namespace, id),
            "signal.sample.v1",
            1,
            fields,
        )?;
        Ok(())
    }

    fn namespace(&self, collection: &str, write: bool) -> std::result::Result<&Namespace, String> {
        let namespace = self
            .namespaces
            .get(collection)
            .ok_or_else(|| format!("unknown collection `{collection}`"))?;
        if !namespace.read || (write && !namespace.write) {
            return Err(format!("denied: `{collection}`"));
        }
        Ok(namespace)
    }

    fn scoped(namespace: &Namespace, id: &str) -> String {
        if namespace.collection.is_empty() {
            id.to_string()
        } else {
            format!("{}:{id}", namespace.collection)
        }
    }

    fn unscoped(namespace: &Namespace, entity_id: &str) -> String {
        if namespace.collection.is_empty() {
            entity_id.to_string()
        } else {
            entity_id
                .strip_prefix(&format!("{}:", namespace.collection))
                .unwrap_or(entity_id)
                .to_string()
        }
    }

    fn entry_value(namespace: &Namespace, record: Record) -> Value {
        let body = serde_json::to_vec(&record.fields).unwrap_or_default();
        Value::List(vec![
            pair(
                "key",
                Value::List(vec![
                    pair("collection", Value::String(namespace.name.clone())),
                    pair(
                        "id",
                        Value::String(Self::unscoped(namespace, &record.entity_id)),
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
        namespace: &Namespace,
        cursor: Option<String>,
        limit: usize,
    ) -> std::result::Result<Value, String> {
        let cursor = cursor.map(|cursor| Self::scoped(namespace, &cursor));
        // A book can hold several collections. Walk a full store page so a
        // sparse collection still fills its requested page; 4096 is the same
        // bound the Soil records host uses.
        let records = self
            .store
            .scan_records(&namespace.book, cursor.as_deref(), 4096)
            .map_err(|error| error.to_string())?;
        let entries: Vec<_> = records
            .into_iter()
            .filter(|record| {
                namespace.collection.is_empty()
                    || record
                        .entity_id
                        .starts_with(&format!("{}:", namespace.collection))
            })
            .take(limit)
            .map(|record| Self::entry_value(namespace, record))
            .collect();
        Ok(page_value(entries))
    }

    fn get_entry(
        &self,
        namespace: &Namespace,
        id: &str,
    ) -> std::result::Result<Option<Value>, String> {
        self.store
            .get_optional(&namespace.book, &Self::scoped(namespace, id))
            .map(|record| record.map(|record| Self::entry_value(namespace, record)))
            .map_err(|error| error.to_string())
    }
}

impl Host for SoilRecordsHost {
    fn granted(&self, interface: &str, function: &str) -> bool {
        if interface != "soil:records" {
            return false;
        }
        let write = matches!(function, "upsert" | "delete" | "apply" | "cas");
        self.namespaces
            .values()
            .any(|namespace| namespace.read && (!write || namespace.write))
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
                let namespace = self.namespace(&collection, false)?;
                Ok(ok(self.scan_page(namespace, cursor, limit)?))
            }
            "get" => {
                let key = args.first().ok_or_else(|| "get needs a key".to_string())?;
                let (collection, id) = parse_key(key)?;
                let namespace = self.namespace(&collection, false)?;
                Ok(match self.get_entry(&namespace, &id)? {
                    Some(entry) => ok(entry),
                    None => err("not-found"),
                })
            }
            "upsert" => {
                let entry = args
                    .first()
                    .cloned()
                    .ok_or_else(|| "upsert needs an entry".to_string())?;
                let key = field_value(&entry, &Value::Keyword("key".to_string()));
                let (collection, id) = parse_key(&key)?;
                let namespace = self.namespace(&collection, true)?.clone();
                let schema = field_value(&entry, &Value::Keyword("schema".to_string()))
                    .as_str()
                    .unwrap_or("signal.sample.v1")
                    .to_string();
                let body = value_bytes(&field_value(&entry, &Value::Keyword("body".to_string())))?;
                let fields: serde_json::Value = serde_json::from_slice(&body)
                    .map_err(|error| format!("invalid JSON body: {error}"))?;
                self.store
                    .put_record(
                        &namespace.book,
                        &Self::scoped(&namespace, &id),
                        &schema,
                        1,
                        fields,
                    )
                    .map_err(|error| error.to_string())?;
                Ok(match self.get_entry(&namespace, &id)? {
                    Some(entry) => ok(entry),
                    None => err("io"),
                })
            }
            "delete" => {
                let key = args
                    .first()
                    .ok_or_else(|| "delete needs a key".to_string())?;
                let (collection, id) = parse_key(key)?;
                let namespace = self.namespace(&collection, true)?.clone();
                let Some(existing) = self.get_entry(&namespace, &id)? else {
                    return Ok(err("not-found"));
                };
                let schema = field_value(&existing, &Value::Keyword("schema".to_string()))
                    .as_str()
                    .unwrap_or("signal.sample.v1")
                    .to_string();
                self.store
                    .delete_record(&namespace.book, &Self::scoped(&namespace, &id), &schema, 1)
                    .map_err(|error| error.to_string())?;
                Ok(ok(Value::Nil))
            }
            "scan-packed" | "cas" => Ok(err("unsupported")),
            other => Err(format!("unknown soil:records function `{other}`")),
        }
    }
}

fn parse_key(value: &Value) -> std::result::Result<(String, String), String> {
    let collection = field_value(value, &Value::Keyword("collection".to_string()))
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| "key needs a collection".to_string())?;
    let id = field_value(value, &Value::Keyword("id".to_string()))
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

fn value_bytes(value: &Value) -> std::result::Result<Vec<u8>, String> {
    match value {
        Value::List(items) => items
            .iter()
            .map(|item| match item {
                Value::Int(byte) if (0..=255).contains(byte) => Ok(*byte as u8),
                _ => Err("body must be a list of bytes".to_string()),
            })
            .collect(),
        Value::String(text) => Ok(text.as_bytes().to_vec()),
        _ => Err("body must be a list of bytes or a string".to_string()),
    }
}

fn field_value(record: &Value, name: &Value) -> Value {
    let Some(items) = record.as_list() else {
        return Value::Nil;
    };
    let key = name.as_keyword().or_else(|| name.as_symbol());
    let Some(key) = key else {
        return Value::Nil;
    };
    for item in items {
        let Some(pair) = item.as_list() else { continue };
        let field = pair
            .first()
            .and_then(|value| value.as_keyword().or_else(|| value.as_symbol()));
        if field == Some(key) {
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
