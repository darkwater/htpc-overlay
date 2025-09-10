use core::ops::Deref;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Root {
    pub device: Device,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[expect(dead_code)]
pub struct Device {
    pub friendly_name: String,
    pub model_name: String,
    pub serial_number: String,
    pub icon_list: IconList,
    pub service_list: ServiceList,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IconList {
    pub icon: Vec<Icon>,
}

impl Deref for IconList {
    type Target = [Icon];

    fn deref(&self) -> &Self::Target {
        &self.icon
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[expect(dead_code)]
pub struct Icon {
    pub mimetype: String,
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceList {
    pub service: Vec<Service>,
}

impl Deref for ServiceList {
    type Target = [Service];

    fn deref(&self) -> &Self::Target {
        &self.service
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[expect(dead_code)]
pub struct Service {
    pub service_type: String,
    pub service_id: String,
    #[serde(rename = "controlURL")]
    pub control_url: String,
    #[serde(rename = "eventSubURL")]
    pub event_sub_url: String,
}
