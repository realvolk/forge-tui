pub mod checklist;
pub mod filter;
pub mod input;
pub mod menu;
pub mod msg;
pub mod password;
pub mod progress;
pub mod summary;
pub mod text;
pub mod yesno;
pub mod disk;

use crate::contract::{Request, Response};
use anyhow::Result;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::fs::File;

pub fn dispatch(request: Request, terminal: Option<&mut Terminal<CrosstermBackend<File>>>) -> Result<Response> {
    match request {
        Request::Menu { title, message, choices, height: _, default, stability_colors, step: _, total: _ } => {
            menu::run(terminal, title, message, choices, default, stability_colors)
        }
        Request::YesNo { title, message, default, step: _, total: _ } => {
            yesno::run(terminal, title, message, default)
        }
        Request::Input { title, message, default, placeholder, validation, step: _, total: _ } => {
            input::run(terminal, title, message, default, placeholder, validation)
        }
        Request::Password { title, message, placeholder, step: _, total: _ } => {
            password::run(terminal, title, message, placeholder)
        }
        Request::Checklist { title, message, choices, height, min, max, default, step: _, total: _ } => {
            checklist::run(terminal, title, message, choices, height, min, max, default)
        }
        Request::Msg { title, message, step: _, total: _ } => {
            msg::run(terminal, title, message)
        }
        Request::Summary { title, message, file, step: _, total: _ } => {
            summary::run(terminal, title, message, file)
        }
        Request::Progress { title, command, logfile, step: _, total: _ } => {
            progress::run(terminal, title, command, logfile)
        }
        Request::Filter { title, message, choices, placeholder, step: _, total: _ } => {
            filter::run(terminal, title, message, choices, placeholder)
        }
        Request::Text { title, file, content, step: _, total: _ } => {
            text::run(terminal, title, file, content)
        }
        Request::Disk { title, disk, partitions, free_space, readonly, step: _, total: _ } => {
            disk::run(terminal, title, disk, partitions, free_space, readonly)
        }
        Request::Quit => {
            Ok(Response { result: None, cancelled: false, error: None })
        }
    }
}