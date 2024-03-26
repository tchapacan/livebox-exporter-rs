use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct Metrics {
    pub status: HashMap<String, DeviceMetrics>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceMetrics {
    #[serde(rename(deserialize = "Traffic"))]
    pub traffic: Vec<TrafficData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TrafficData {
    #[serde(rename(deserialize = "Rx_Counter"))]
    pub rx_counter: u64,
    #[serde(rename(deserialize = "Tx_Counter"))]
    pub tx_counter: u64,
    #[serde(rename(deserialize = "Timestamp"))]
    pub timestamp: u64,
}
