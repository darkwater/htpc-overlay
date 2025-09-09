use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::time::Time;

#[derive(Serialize)]
pub struct Command {
    command: Value,
}

impl Command {
    pub fn observe_property(id: i32, property: &str) -> Self {
        Command {
            command: json!(["observe_property", id, property]),
        }
    }

    pub fn set_property(name: &str, value: impl Serialize) -> Self {
        let value = serde_json::to_value(value).expect("value to be serializable");

        Command {
            command: serde_json::json!(["set_property", name, value]),
        }
    }

    pub fn cycle_property(name: &str) -> Self {
        Command {
            command: serde_json::json!(["cycle", name]),
        }
    }

    pub fn add_property(name: &str, value: f32) -> Self {
        Command {
            command: serde_json::json!(["add", name, value]),
        }
    }

    pub fn seek(seconds: Time, exact: bool) -> Command {
        Command {
            command: json!(["seek", seconds, if exact { "exact" } else { "keyframes" }]),
        }
    }
}

#[derive(Deserialize)]
pub struct Response<T> {
    pub error: String,
    pub data: Option<T>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
#[serde(tag = "event")]
pub enum Event {
    PropertyChange {
        #[serde(default)]
        data: Value,
        name: String,
    },
    Seek,
    #[serde(other)]
    Unknown,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum EventOrResponse<T> {
    Event(Event),
    Response(Response<T>),
}
