use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
#[serde(tag = "widget")]
pub enum Request {
    #[serde(rename = "menu")]
    Menu {
        #[serde(default)]
        title: String,
        #[serde(default)]
        message: String,
        #[serde(default)]
        choices: Value,
        #[serde(default)]
        height: Option<u16>,
        #[serde(default)]
        default: Option<String>,
        #[serde(default)]
        stability_colors: Option<HashMap<String, String>>,
    },
    #[serde(rename = "yesno")]
    YesNo {
        #[serde(default)]
        title: String,
        #[serde(default)]
        message: String,
        #[serde(default)]
        default: Option<bool>,
    },
    #[serde(rename = "input")]
    Input {
        #[serde(default)]
        title: String,
        #[serde(default)]
        message: String,
        #[serde(default)]
        default: Option<String>,
        #[serde(default)]
        placeholder: Option<String>,
        #[serde(default)]
        validation: Option<String>,
    },
    #[serde(rename = "password")]
    Password {
        #[serde(default)]
        title: String,
        #[serde(default)]
        message: String,
        #[serde(default)]
        placeholder: Option<String>,
    },
    #[serde(rename = "checklist")]
    Checklist {
        #[serde(default)]
        title: String,
        #[serde(default)]
        message: String,
        choices: Vec<String>,
        #[serde(default)]
        height: Option<u16>,
        #[serde(default)]
        min: Option<usize>,
        #[serde(default)]
        max: Option<usize>,
        #[serde(default)]
        default: Option<Vec<String>>,
    },
    #[serde(rename = "msg")]
    Msg {
        #[serde(default)]
        title: String,
        message: String,
    },
    #[serde(rename = "summary")]
    Summary {
        #[serde(default)]
        title: String,
        #[serde(default)]
        message: Option<String>,
        #[serde(default)]
        file: Option<String>,
    },
    #[serde(rename = "progress")]
    Progress {
        #[serde(default)]
        title: String,
        command: Vec<String>,
        #[serde(default)]
        logfile: Option<String>,
    },
    #[serde(rename = "filter")]
    Filter {
        #[serde(default)]
        title: String,
        #[serde(default)]
        message: String,
        choices: Vec<String>,
        #[serde(default)]
        placeholder: Option<String>,
    },
}

#[derive(Debug, Serialize)]
pub struct Response {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(default)]
    pub cancelled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}