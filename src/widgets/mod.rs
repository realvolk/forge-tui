pub mod menu;
pub mod yesno;
pub mod input;
pub mod password;
pub mod checklist;
pub mod msg;
pub mod summary;
pub mod progress;
pub mod filter;

use crate::contract::{Request, Response};
use anyhow::Result;

pub fn dispatch(request: Request) -> Result<Response> {
    match request {
        Request::Menu { title, message, choices, height: _, default, stability_colors } => {
            menu::run(title, message, choices, default, stability_colors)
        }
        Request::YesNo { title, message, default } => {
            yesno::run(title, message, default)
        }
        Request::Input { title, message, default, placeholder, validation } => {
            input::run(title, message, default, placeholder, validation)
        }
        Request::Password { title, message, placeholder } => {
            password::run(title, message, placeholder)
        }
        Request::Checklist { title, message, choices, height, min, max, default } => {
            checklist::run(title, message, choices, height, min, max, default)
        }
        Request::Msg { title, message } => {
            msg::run(title, message)
        }
        Request::Summary { title, message, file } => {
            summary::run(title, message, file)
        }
        Request::Progress { title, command, logfile } => {
            progress::run(title, command, logfile)
        }
        Request::Filter { title, message, choices, placeholder } => {
            filter::run(title, message, choices, placeholder)
        }
    }
}