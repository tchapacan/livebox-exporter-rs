
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all="PascalCase")]
pub struct Device {
    pub key: String,
    pub name: String,
    pub discovery_source: String,
    pub active: bool,
    pub device_type: String,
    pub tags: String,
    #[serde(rename(deserialize = "IPAddress"))]
    pub ip_address: Option<String>,
    #[serde(rename(deserialize = "SSID"))]
    pub ssid: Option<String>,
    pub channel: Option<u32>,
}
