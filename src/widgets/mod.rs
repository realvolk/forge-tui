pub mod checklist;
pub mod disk;
pub mod file_picker;
pub mod filter;
pub mod helpers;
pub mod hub;
pub mod input;
pub mod menu;
pub mod msg;
pub mod multiselect;
pub mod password;
pub mod progress;
pub mod summary;
pub mod text;
pub mod yesno;

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
        Request::Multiselect { title, message, choices, placeholder, min, max, step: _, total: _ } => {
            multiselect::run(terminal, title, message, choices, placeholder, min, max)
        }
        Request::Text { title, file, content, step: _, total: _ } => {
            text::run(terminal, title, file, content)
        }
        Request::Disk { title, disk, partitions, free_space, readonly, step: _, total: _ } => {
            disk::run(terminal, title, disk, partitions, free_space, readonly)
        }
        Request::Hub { title, categories, actions, step: _, total: _ } => {
            hub::run(terminal, title, categories, actions)
        }
        Request::FilePicker { title, start_dir, filter } => {
            file_picker::run(terminal, title, start_dir, filter)
        }
        Request::Recovery { title, status, repairs } => {
            crate::artixforge::recovery::hub::run(terminal, title, status, repairs)
        }
        Request::Iso { title, categories } => {
            crate::artixforge::iso::hub::run(terminal, title, categories)
        }
        Request::MigrationInit { title, current_init } => {
            crate::artixforge::migration::init::run(terminal, title, current_init)
        }
        Request::MigrationDesktop { title, current_de } => {
            crate::artixforge::migration::desktop::run(terminal, title, current_de)
        }
        Request::Anvil { title, actions } => {
            crate::artixforge::anvil::hub::run(terminal, title, actions)
        }
        Request::PowerUser { title, categories } => {
            crate::artixforge::poweruser::hub::run(terminal, title, categories)
        }
        Request::Quit => Ok(Response { result: None, cancelled: false, error: None }),
    }
}