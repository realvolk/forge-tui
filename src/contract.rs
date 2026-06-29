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
        #[serde(default)]
        step: Option<u16>,
        #[serde(default)]
        total: Option<u16>,
    },
    #[serde(rename = "yesno")]
    YesNo {
        #[serde(default)]
        title: String,
        #[serde(default)]
        message: String,
        #[serde(default)]
        default: Option<bool>,
        #[serde(default)]
        step: Option<u16>,
        #[serde(default)]
        total: Option<u16>,
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
        #[serde(default)]
        step: Option<u16>,
        #[serde(default)]
        total: Option<u16>,
    },
    #[serde(rename = "password")]
    Password {
        #[serde(default)]
        title: String,
        #[serde(default)]
        message: String,
        #[serde(default)]
        placeholder: Option<String>,
        #[serde(default)]
        step: Option<u16>,
        #[serde(default)]
        total: Option<u16>,
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
        #[serde(default)]
        step: Option<u16>,
        #[serde(default)]
        total: Option<u16>,
    },
    #[serde(rename = "msg")]
    Msg {
        #[serde(default)]
        title: String,
        message: String,
        #[serde(default)]
        step: Option<u16>,
        #[serde(default)]
        total: Option<u16>,
    },
    #[serde(rename = "summary")]
    Summary {
        #[serde(default)]
        title: String,
        #[serde(default)]
        message: Option<String>,
        #[serde(default)]
        file: Option<String>,
        #[serde(default)]
        step: Option<u16>,
        #[serde(default)]
        total: Option<u16>,
    },
    #[serde(rename = "progress")]
    Progress {
        #[serde(default)]
        title: String,
        command: Vec<String>,
        #[serde(default)]
        logfile: Option<String>,
        #[serde(default)]
        step: Option<u16>,
        #[serde(default)]
        total: Option<u16>,
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
        #[serde(default)]
        step: Option<u16>,
        #[serde(default)]
        total: Option<u16>,
    },
    #[serde(rename = "text")]
    Text {
        #[serde(default)]
        title: String,
        #[serde(default)]
        file: Option<String>,
        #[serde(default)]
        content: Option<String>,
        #[serde(default)]
        step: Option<u16>,
        #[serde(default)]
        total: Option<u16>,
    },
    #[serde(rename = "disk")]
    Disk {
        #[serde(default)]
        title: String,
        #[serde(default)]
        disk: String,
        #[serde(default)]
        partitions: Value,
        #[serde(default)]
        free_space: Option<Value>,
        #[serde(default)]
        readonly: Option<bool>,
        #[serde(default)]
        step: Option<u16>,
        #[serde(default)]
        total: Option<u16>,
    },
    #[serde(rename = "quit")]
    Quit,
}

impl Request {
    pub fn step(&self) -> u16 {
        match self {
            Request::Menu { step, .. } => step.unwrap_or(0),
            Request::YesNo { step, .. } => step.unwrap_or(0),
            Request::Input { step, .. } => step.unwrap_or(0),
            Request::Password { step, .. } => step.unwrap_or(0),
            Request::Checklist { step, .. } => step.unwrap_or(0),
            Request::Msg { step, .. } => step.unwrap_or(0),
            Request::Summary { step, .. } => step.unwrap_or(0),
            Request::Progress { step, .. } => step.unwrap_or(0),
            Request::Filter { step, .. } => step.unwrap_or(0),
            Request::Text { step, .. } => step.unwrap_or(0),
            Request::Disk { step, .. } => step.unwrap_or(0),
            Request::Quit => 0,
        }
    }

    pub fn total(&self) -> u16 {
        match self {
            Request::Menu { total, .. } => total.unwrap_or(0),
            Request::YesNo { total, .. } => total.unwrap_or(0),
            Request::Input { total, .. } => total.unwrap_or(0),
            Request::Password { total, .. } => total.unwrap_or(0),
            Request::Checklist { total, .. } => total.unwrap_or(0),
            Request::Msg { total, .. } => total.unwrap_or(0),
            Request::Summary { total, .. } => total.unwrap_or(0),
            Request::Progress { total, .. } => total.unwrap_or(0),
            Request::Filter { total, .. } => total.unwrap_or(0),
            Request::Text { total, .. } => total.unwrap_or(0),
            Request::Disk { total, .. } => total.unwrap_or(0),
            Request::Quit => 0,
        }
    }
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