use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Metadata {
    pub name: String,
}

impl Default for Metadata {
    fn default() -> Self {
        Self {
            name: "Untitled scene".to_owned(),
        }
    }
}
