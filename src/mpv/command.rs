use std::io;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

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

    pub fn set_property(name: &str, value: impl Into<Value>) -> Self {
        Command {
            command: serde_json::json!(["set_property", name, value.into()]),
        }
    }

    pub fn cycle_property(name: &str) -> Self {
        Command {
            command: serde_json::json!(["cycle", name]),
        }
    }

    pub fn write_to(self, stream: &mut impl io::Write) -> io::Result<()> {
        let cmd_str = serde_json::to_string(&self.command)?;
        stream.write_all(cmd_str.as_bytes())
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
        data: Value,
        name: String,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum EventOrResponse<T> {
    Event(Event),
    Response(Response<T>),
}
