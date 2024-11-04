use serde::{Serialize, Deserialize};

pub type Root = Vec<Root2>;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root2 {
    #[serde(default)]
    pub executables: Vec<Executable>,
    pub hook: bool,
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub overlay: Option<bool>,
    #[serde(rename = "overlay_compatibility_hook")]
    pub overlay_compatibility_hook: Option<bool>,
    #[serde(rename = "overlay_methods")]
    pub overlay_methods: Option<i64>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Executable {
    #[serde(rename = "is_launcher")]
    pub is_launcher: bool,
    pub name: String,
    pub os: String,
    pub arguments: Option<String>,
}
