use std::collections::BTreeMap;
use std::{fs, io};
use std::path::Path;
use mc_world_parser::Block;
use mc_world_parser::section::BlockIDGetter;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
pub struct BlockStateDefinition {
    r#type: String,
    properties: Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BlockState {
    default: Option<bool>,
    id: i32,
    #[serde(default)]
    properties: BTreeMap<String, String>
}

#[derive(Debug, Clone, Deserialize)]
pub struct BlockStates {
    definition: BlockStateDefinition,
    #[serde(default)]
    properties: BTreeMap<String, Vec<String>>,
    states: Vec<BlockState>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BlockRegistry {
    blocks: BTreeMap<String, BlockStates>
}

impl BlockIDGetter for BlockRegistry {
    fn id_of(&self, block: &Block) -> i32 {
        self.get_blockstate_of_block(block).unwrap_or(0)
    }
}

impl BlockRegistry {
    pub fn load<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let json = fs::read_to_string(path)?;

        let blocks: BTreeMap<String, BlockStates> = serde_json::from_str(&*json).unwrap();
        Ok(Self {blocks})
    }

    pub fn get_blockstate_of_block(&self, block: &Block) -> Option<i32> {
        for (name, states) in &self.blocks {
            if block.identifier().eq(name) {
                for state in &states.states {
                    if let Some(default) = state.default {
                        if default {
                            return Some(state.id);
                        }
                    }
                }
            }
        }
        None
    }
}