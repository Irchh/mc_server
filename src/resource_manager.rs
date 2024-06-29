use std::collections::BTreeMap;
use std::io;
use std::path::Path;
use std::str::FromStr;
use log::debug;
use serde_json::Value;
use walkdir::WalkDir;
use crate::server_util::{RegistryEntry, TagEntry};

pub struct ResourceManager {
    registries: BTreeMap<String, Vec<RegistryEntry>>,
    tags: Vec<TagEntry>,
}

impl ResourceManager {
    pub fn new<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        // TODO: Refactor for error resilience
        //let root_dir = std::fs::read_dir(path)?;
        let mut registries = BTreeMap::new();
        let registries_dir = WalkDir::new(path.as_ref().join("registries"));
        for entry in registries_dir.into_iter().filter_map(|e| e.ok()) {
            let full_entry_name = entry.path().strip_prefix(path.as_ref().join("registries")).unwrap();
            debug!("Entry path: {}", full_entry_name.display());
            let mut name_iter = full_entry_name.iter();
            let next = name_iter.next();
            if next.is_none() {
                continue;
            }
            let namespace = next.unwrap().to_str().unwrap().to_string();
            if namespace.len() == 0 {
                continue;
            }
            let id = full_entry_name.strip_prefix(namespace.clone()).unwrap().as_os_str().to_str().unwrap().to_string();
            if !id.ends_with(".json") {
                continue;
            }

            let file_data = std::fs::read_to_string(entry.path()).unwrap();
            let json = Value::from_str(&*file_data).unwrap();
            let entries = Self::json_to_registry_entries(json);

            registries.insert(namespace + ":" + id.strip_suffix(".json").unwrap(), entries);
        }

        let file_data = std::fs::read_to_string("resources/tags/minecraft/tags.json").unwrap();
        let json = Value::from_str(&*file_data).unwrap();
        let tags = Self::json_to_tags(json);

        Ok(Self {
            registries,
            tags,
        })
    }

    pub fn registries_ref(&self) -> &BTreeMap<String, Vec<RegistryEntry>> {
        &self.registries
    }

    fn json_to_registry_entries(json: Value) -> Vec<RegistryEntry> {
        let mut entries = vec![];
        let map = json.get("entries").unwrap().as_array().unwrap();
        for val in map {
            let id = val.get("id").unwrap();
            let namespace = id.get("namespace").unwrap().as_str().unwrap();
            let name = id.get("name").unwrap().as_str().unwrap();
            entries.push(RegistryEntry {
                id: namespace.to_string() + ":" + name,
                data: None,
            });
        }
        entries
    }

    fn json_to_tags(json: Value) -> Vec<TagEntry> {
        let mut entries = vec![];
        let map = json.get("tags").unwrap().as_object().unwrap();
        for (id, val) in map {
            entries.push(TagEntry {
                id: id.clone(),
                data: None,
            })
        }
        entries
    }
}