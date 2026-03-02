use super::types::Universe;

pub struct UniverseAllowlist;

impl UniverseAllowlist {
    pub fn load_from_disk() -> Universe {
        let chains: Vec<String> =
            serde_json::from_str(&std::fs::read_to_string("universe/allowed_chains.json").unwrap())
                .unwrap();
        let dexes: Vec<String> =
            serde_json::from_str(&std::fs::read_to_string("universe/allowed_dexes.json").unwrap())
                .unwrap();
        let tokens: Vec<String> =
            serde_json::from_str(&std::fs::read_to_string("universe/allowed_tokens.json").unwrap())
                .unwrap();

        Universe {
            allowed_chains: chains,
            allowed_dexes: dexes,
            allowed_tokens: tokens,
        }
    }
}
