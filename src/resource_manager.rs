use std::collections::BTreeMap;
use std::ffi::OsString;
use std::{fs, io};
use std::path::Path;
use std::str::FromStr;
use log::{debug, error, trace};
use serde_json::{to_string, Value};
use walkdir::WalkDir;
use crate::block_registry::BlockRegistry;
use crate::error::ServerError;
use crate::server_util::{RegistryEntry, TagEntry, TagEntryData};

#[derive(Debug, Clone)]
pub struct ResourceManager {
    registries: BTreeMap<String, Vec<RegistryEntry>>,
    block_registry: BlockRegistry,
    tags: Vec<TagEntry>,
}

impl ResourceManager {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, ServerError> {
        let registries = Self::build_registries(path.as_ref())?;
        let tags = Self::build_tags(path.as_ref())?;
        Ok(Self {
            registries,
            block_registry: BlockRegistry::load(path.as_ref().join("generated/reports/blocks.json"))?,
            tags,
        })
    }

    fn build_registries<P: AsRef<Path>>(path: P) -> Result<BTreeMap<String, Vec<RegistryEntry>>, ServerError> {
        let mut registries = BTreeMap::new();

        // TODO: Check which directories should be included instead
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
            registry_name = "minecraft:".to_string() + registry_name.as_str();

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
        Ok(registries)
    }

    fn build_tags<P: AsRef<Path>>(path: P) -> Result<Vec<TagEntry>, ServerError> {
        let mut tags: Vec<TagEntry> = vec![];
        let tags_dir = WalkDir::new(path.as_ref().join("generated/data/minecraft/tags"));
        for entry in tags_dir.into_iter().filter_map(|e| e.ok()).filter(|e| e.path().extension().eq(&Some(&*OsString::from("json")))) {
            let full_entry_name = entry.path().strip_prefix(path.as_ref().join("generated/data/minecraft/tags"))?;
            debug!("Tag entry path: {}", full_entry_name.display());
            let mut name_iterator = full_entry_name.iter();
            let tag_registry_name = "minecraft:".to_string() + name_iterator.next().unwrap().to_str().unwrap();
            let tag_entry_parts = name_iterator.map(|s| s.to_str().unwrap()).collect::<Vec<_>>();
            let mut tag_name = "".to_string();
            for (i, part) in tag_entry_parts.iter().enumerate() {
                tag_name += part;
                if i != tag_entry_parts.len() - 1 {
                    tag_name += "/";
                }
            }
            let file_contents = fs::read_to_string(entry.path())?;
            let parsed = Value::from_str(&file_contents).unwrap();
            let values = parsed.get("values").unwrap().as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect::<Vec<_>>();

            tag_name = tag_name.strip_suffix(".json").unwrap_or(tag_name.as_str()).to_string();
            error!("[{}] {}", tag_registry_name, tag_name);

            let tag_entry_data = TagEntryData {
                entries: values,
                tag_name,
            };

            if let Some(tag_registry) = tags.iter_mut().find(|e| e.id == tag_registry_name) {
                tag_registry.data.push(tag_entry_data)
            } else {
                tags.push(TagEntry {
                    id: tag_registry_name,
                    data: vec![tag_entry_data],
                });
            }
        }

        let file_data = fs::read_to_string("resources/tags/minecraft/tags.json").unwrap();
        let json = Value::from_str(&*file_data).unwrap();

        Ok(tags)
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
}