use std::collections::BTreeMap;
use std::io;
use std::path::Path;
use std::str::FromStr;
use log::debug;
use serde_json::Value;
use walkdir::WalkDir;
use crate::block_registry::BlockRegistry;
use crate::server_util::{RegistryEntry, TagEntry, TagEntryData};

pub struct ResourceManager {
    registries: BTreeMap<String, Vec<RegistryEntry>>,
    block_registry: BlockRegistry,
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
            block_registry: BlockRegistry::load("resources/blocks.json")?,
            tags,
        })
    }

    pub fn registries_ref(&self) -> &BTreeMap<String, Vec<RegistryEntry>> {
        &self.registries
    }

    pub fn block_registry_ref(&self) -> &BlockRegistry {
        &self.block_registry
    }

    pub fn tags_ref(&self) -> &Vec<TagEntry> {
        &self.tags
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
        // TODO: Fix these variable names
        let mut entries = vec![];
        let map = json.get("tags").unwrap().as_object().unwrap();
        for (id, val) in map {
            let tag_entry_data_arr = val.as_array().cloned().unwrap();
            let mut data = vec![];
            for entry_data in tag_entry_data_arr {
                let entry_entries = entry_data.get("entries").unwrap().as_array().unwrap().iter().map(|v| v.as_i64().unwrap() as i32).collect::<Vec<i32>>();
                let tag_name = entry_data.get("tag_name").unwrap();
                data.push(TagEntryData {
                    entries: entry_entries,
                    tag_name: tag_name.get("namespace").unwrap().as_str().unwrap().to_string() + ":" + tag_name.get("name").unwrap().as_str().unwrap(),
                });
            }
            entries.push(TagEntry {
                id: id.clone(),
                data
            })
        }
        entries
    }
}