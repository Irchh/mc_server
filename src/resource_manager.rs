use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io;
use std::path::Path;
use std::str::FromStr;
use log::{debug, error, trace};
use serde_json::Value;
use walkdir::WalkDir;
use crate::block_registry::BlockRegistry;
use crate::error::ServerError;
use crate::server_util::{RegistryEntry, TagEntry, TagEntryData};

pub struct ResourceManager {
    registries: BTreeMap<String, Vec<RegistryEntry>>,
    block_registry: BlockRegistry,
    tags: Vec<TagEntry>,
}

impl ResourceManager {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, ServerError> {
        let mut registries = BTreeMap::new();

        let excluded_dirs = vec!["minecraft/tags", "minecraft/datapacks", "minecraft/loot_table", "minecraft/recipe", "minecraft/advancement"];

        let registries_dir = WalkDir::new(path.as_ref().join("generated/data"));
        'outer: for entry in registries_dir.into_iter().filter_map(|e| e.ok()).filter(|e| e.path().extension().eq(&Some(&*OsString::from("json")))) {
            let full_entry_name = entry.path().strip_prefix(path.as_ref().join("generated/data"))?;
            for dir in &excluded_dirs {
                if full_entry_name.starts_with(dir) {
                    continue 'outer;
                }
            }
            debug!("Registry entry path: {}", full_entry_name.display());
            let mut name_iterator = full_entry_name.iter();
            let namespace = name_iterator.next().unwrap().to_str().unwrap();
            let registry_name_parts = name_iterator.clone().take(name_iterator.clone().collect::<Vec<_>>().len()-1).map(|s| s.to_str().unwrap()).collect::<Vec<_>>();
            let mut registry_name = "".to_string();
            for (i, part) in registry_name_parts.iter().enumerate() {
                registry_name += part;
                if i != registry_name_parts.len() - 1 {
                    registry_name += "/";
                }
            }
            let identifier = name_iterator.as_path().file_stem().unwrap().to_str().unwrap();

            trace!("[{}] {}:{}", registry_name, namespace, identifier);
            if !registries.contains_key(&registry_name) {
                registries.insert(registry_name.clone(), vec![]);
            }
            let registry = registries.get_mut(&registry_name).unwrap();
            registry.push(RegistryEntry {
                id: namespace.to_string() + ":" + identifier,
                data: None,
            })
        }

        let file_data = std::fs::read_to_string("resources/tags/minecraft/tags.json").unwrap();
        let json = Value::from_str(&*file_data).unwrap();
        let tags = Self::json_to_tags(json);

        Ok(Self {
            registries,
            block_registry: BlockRegistry::load(path.as_ref().join("generated/reports/blocks.json"))?,
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