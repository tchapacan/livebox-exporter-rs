use crate::{
    livebox_client_rs::devices::Device,
    livebox_client_rs::metrics::{DeviceMetrics, Metrics},
    livebox_client_rs::status::Status,
    livebox_client_rs::wan::WANConfiguration,
};
use cookie::Cookie;
use hyper::{
    body::{Body, Bytes},
    client::HttpConnector,
    header::{AUTHORIZATION, CONTENT_TYPE, COOKIE, SET_COOKIE},
    Method, Request,
};
use log::{debug, trace};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::net::Ipv4Addr;

#[derive(Debug, Clone)]
pub struct Client {
    ip: String,
    username: String,
    password: String,
    cookies: Vec<String>,
    context_id: Option<String>,
    client: hyper::Client<HttpConnector>,
}

impl Client {
    pub fn new(password: &str) -> Self {
        trace!("Creating a new client.");
        assert!(!password.is_empty(), "Password is empty!");
        Self {
            ip: Ipv4Addr::new(192, 168, 1, 1).to_string(),
            username: "admin".to_string(),
            password: password.to_string(),
            cookies: Vec::new(),
            context_id: None,
            client: hyper::Client::new(),
        }
    }

    async fn post_request(
        &self,
        service: &str,
        method: &str,
        parameters: serde_json::Value,
    ) -> (hyper::http::response::Parts, Bytes) {
        let post_data = json!({
            "service": service,
            "method": method,
            "parameters": parameters
        });
        let req = Request::builder()
            .method(Method::POST)
            .uri(format!("http://{}/ws", self.ip))
            .header(CONTENT_TYPE, "application/x-sah-ws-4-call+json")
            .header(AUTHORIZATION, "X-Sah-Login")
            .body(Body::from(post_data.to_string()))
            .expect("Could not build request.");
        let (parts, body) = self
            .client
            .request(req)
            .await
            .expect("There was an issue contacting the router.")
            .into_parts();
        let body_bytes = hyper::body::to_bytes(body).await.unwrap();
        debug!("Status is {}.", parts.status.as_str());
        (parts, body_bytes)
    }

    pub async fn login(&mut self) {
        trace!("Logging in.");
        let (parts, body_bytes) = self
            .post_request(
                "sah.Device.Information",
                "createContext",
                serde_json::json!({
                    "applicationName": "so_sdkut",
                    "username": &self.username,
                    "password": &self.password
                }),
            )
            .await;
        debug!("Status is {}.", parts.status.as_str());
        for ele in parts.headers.get_all(SET_COOKIE) {
            let cookie = Cookie::parse(ele.to_str().unwrap()).unwrap();
            self.cookies
                .push(format!("{}={}", cookie.name(), cookie.value()));
        }
        assert!(
            !self.cookies.is_empty(),
            "No cookie detected on login, there should be an error."
        );
        let json: serde_json::Value =
            serde_json::from_slice(&body_bytes).expect("Could not parse JSON.");
        assert_eq!(json["status"].as_u64().unwrap(), 0, "Status wasn't 0.");
        self.context_id = Some(json["data"]["contextID"].as_str().unwrap().to_string());
    }

    async fn authenticated_post_request(
        &self,
        service: &str,
        method: &str,
        parameters: serde_json::Value,
    ) -> (hyper::http::response::Parts, Bytes) {
        let post_data = json!({
            "service": service,
            "method": method,
            "parameters": parameters
        });
        assert!(
            self.context_id.is_some(),
            "Cannot make authenticated request without logging in beforehand."
        );
        let req = Request::builder()
            .method(Method::POST)
            .uri(format!("http://{}/ws", self.ip))
            .header(CONTENT_TYPE, "application/x-sah-ws-4-call+json")
            .header("X-Context", self.context_id.clone().unwrap())
            .header(COOKIE, self.cookies.join("; "))
            .body(Body::from(post_data.to_string()))
            .expect("Could not build request.");
        let (parts, body) = self
            .client
            .request(req)
            .await
            .expect("There was an issue contacting the router.")
            .into_parts();
        let body_bytes = hyper::body::to_bytes(body).await.unwrap();
        debug!("Status is {}.", parts.status.as_str());
        (parts, body_bytes)
    }

    pub async fn get_status(&self) -> Status {
        let (parts, body_bytes) = self
            .authenticated_post_request("DeviceInfo", "get", serde_json::json!({}))
            .await;
        let json: Value = serde_json::from_slice(&body_bytes).expect("Could not parse JSON.");
        println!("Body was: '{}'.", std::str::from_utf8(&body_bytes).unwrap());
        assert!(
            parts.status.is_success(),
            "Router answered with something else than a success code."
        );
        let status: Status = serde_json::from_value(json["status"].clone())
            .expect("Looks like the deserialized data is incomplete.");
        debug!("Deserialized status is: {:?}", status);
        status
    }

    pub async fn get_wan_config(&self) -> WANConfiguration {
        let (parts, body_bytes) = self
            .authenticated_post_request("NMC", "getWANStatus", serde_json::json!({}))
            .await;
        let json: Value = serde_json::from_slice(&body_bytes).expect("Could not parse JSON.");
        println!("Body was: '{}'.", std::str::from_utf8(&body_bytes).unwrap());
        assert!(
            parts.status.is_success(),
            "Router answered with something else than a success code."
        );
        let wan_config: WANConfiguration = serde_json::from_value(json["data"].clone())
            .expect("Looks like the deserialized data is incomplete.");
        debug!("Deserialized wan is: {:?}", wan_config);
        wan_config
    }

    pub async fn get_devices(&self) -> Vec<Device> {
        let (parts, body_bytes) = self
            .authenticated_post_request("Devices", "get", serde_json::json!({}))
            .await;
        let json: Value = serde_json::from_slice(&body_bytes).expect("Could not parse JSON.");
        assert!(
            parts.status.is_success() && json["status"].is_array(),
            "Router answered with something else than a success code."
        );
        let devices: Vec<Device> = serde_json::from_value(json["status"].clone())
            .expect("Looks like the deserialized data is incomplete.");
        debug!("Deserialized devices is: {:?}", devices);
        devices
    }

    pub async fn get_metrics(&self) -> Vec<Metrics> {
        let post_data = json!({"Seconds": 0, "NumberOfReadings": 1});
        let (_parts, body_bytes) = self
            .authenticated_post_request("HomeLan", "getResults", post_data)
            .await;
        let json: Value = serde_json::from_slice(&body_bytes).expect("Could not parse JSON.");
        println!("Body was: '{}'.", std::str::from_utf8(&body_bytes).unwrap());
        let mut metrics: Vec<Metrics> = Vec::new();
        if let Some(status) = json["status"].as_object() {
            for (key, value) in status.iter() {
                let device_metrics: DeviceMetrics = serde_json::from_value(value.clone())
                    .expect("Error deserializing DeviceMetrics");
                let mut status_map: HashMap<String, DeviceMetrics> = HashMap::new();
                status_map.insert(key.clone(), device_metrics);
                metrics.push(Metrics { status: status_map });
            }
        }
        debug!("Deserialized metrics is: {:?}", metrics);
        metrics
    }

    pub async fn logout(&mut self) {
        trace!("Logging out.");
        let post_data = json!({
            "service":"sah.Device.Information",
            "method":"releaseContext",
            "parameters":{"applicationName":"so_sdkut"}
        });
        let req = Request::builder()
            .method(Method::POST)
            .uri(format!("http://{}/ws", self.ip))
            .header(
                AUTHORIZATION,
                format!("X-Sah-Logout {}", self.context_id.clone().unwrap()),
            )
            .header(COOKIE, self.cookies.join("; "))
            .body(Body::from(post_data.to_string()))
            .expect("Could not build request.");
        self.client
            .request(req)
            .await
            .expect("There was an issue contacting the router.");
        trace!("Logged out.");
        self.cookies.clear();
        self.context_id = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::{Method::POST, MockServer};
    use serde_json::json;

    fn get_mock_status() -> &'static str {
        r#"{
            "status": {
                "Manufacturer": "test",
                "ManufacturerOUI": "test",
                "ModelName": "test",
                "Description": "test", 
                "ProductClass": "test",
                "SerialNumber": "test",
                "HardwareVersion": "test",
                "SoftwareVersion": "test",
                "RescueVersion": "test",
                "ModemFirmwareVersion": "test",
                "EnabledOptions": "test",
                "AdditionalHardwareVersion": "test",
                "AdditionalSoftwareVersion": "test",
                "SpecVersion": "test",
                "ProvisioningCode": "test",
                "UpTime": 0,
                "FirstUseDate": "test",
                "DeviceLog": "test",
                "VendorConfigFileNumberOfEntries": 0,
                "ManufacturerURL": "test",
                "Country": "test",
                "ExternalIPAddress": "test",
                "DeviceStatus": "test",
                "NumberOfReboots": 0,
                "UpgradeOccurred": false,
                "ResetOccurred": false,
                "RestoreOccurred": false,
                "StandbyOccurred": false,
                "X_SOFTATHOME-COM_AdditionalSoftwareVersions": "test",
                "BaseMAC": "test"
            }
        }"#
    }

    fn get_mock_wan_config() -> &'static str {
        r#"{
            "data": {
                "WanState": "test",
                "LinkType": "test",
                "LinkState": "test",
                "MACAddress": "test",
                "Protocol": "test",
                "ConnectionState": "test",
                "LastConnectionError": "test",
                "IPAddress": "test",
                "RemoteGateway": "test",
                "DNSServers": "test",
                "IPv6Address": "test",
                "IPv6DelegatedPrefix": "test"
            }
        }"#
    }

    fn get_mock_devices() -> &'static str {
        r#"{
            "status": [{
                "Key": "test",
                "Name": "test",
                "DiscoverySource": "test",
                "Active": true,
                "DeviceType": "test",
                "Tags": "test",
                "IPAddress": "test",
                "SSID": "test",
                "Channel": 11
            }]
        }"#
    }

    fn get_mock_metrics() -> &'static str {
        r#"{
            "status":{
                "test":{
                    "Traffic":[{
                        "Timestamp":1711483314,
                        "Rx_Counter":1259440,
                        "Tx_Counter":9696752
                    }]
                }
            }
        }"#
    }

    #[tokio::test]
    async fn test_client_instantiation() {
        let password = "test_password";
        let client = Client::new(password);
        assert_eq!(client.ip, "192.168.1.1");
        assert_eq!(client.username, "admin");
        assert_eq!(client.password, password);
        assert!(client.cookies.is_empty());
        assert!(client.context_id.is_none());
    }

    #[tokio::test]
    async fn test_login_success() {
        let server = MockServer::start();
        let _m = server.mock(|when, then| {
            when.method(POST)
                .path("/ws")
                .header("content-type", "application/x-sah-ws-4-call+json")
                .header("authorization", "X-Sah-Login");
            then.status(200)
                .header("set-cookie", "session=mocked_session_id")
                .body(json!({"status": 0, "data": {"contextID": "test-context-id"}}).to_string());
        });
        let mut client = Client::new("password");
        client.ip = server.address().to_string();
        client.login().await;
        assert_eq!(client.cookies.len(), 1);
        assert_eq!(client.context_id, Some("test-context-id".to_string()));
    }

    #[tokio::test]
    async fn test_get_status() {
        let server = MockServer::start();
        let mock_status = get_mock_status();
        let _m = server.mock(|when, then| {
            when.method(POST)
                .path("/ws")
                .header("x-context", "test-context-id");
            then.status(200).body(mock_status);
        });
        let mut client = Client::new("password");
        client.ip = server.address().to_string();
        client.cookies.push("session=mocked_session_id".to_string());
        client.context_id = Some("test-context-id".to_string());
        let status = client.get_status().await;
        assert_eq!(status.manufacturer, "test");
    }

    #[tokio::test]
    async fn test_get_wan_config() {
        let server = MockServer::start();
        let mock_wan_config = get_mock_wan_config();
        let _m = server.mock(|when, then| {
            when.method(POST)
                .path("/ws")
                .header("x-context", "test-context-id");
            then.status(200).body(mock_wan_config);
        });
        let mut client = Client::new("password");
        client.ip = server.address().to_string();
        client.cookies.push("session=mocked_session_id".to_string());
        client.context_id = Some("test-context-id".to_string());
        let wan: WANConfiguration = client.get_wan_config().await;
        assert_eq!(wan.wan_state, "test");
    }

    #[tokio::test]
    async fn test_get_devices() {
        let server = MockServer::start();
        let mock_devices = get_mock_devices();
        let _m = server.mock(|when, then| {
            when.method(POST)
                .path("/ws")
                .header("x-context", "test-context-id");
            then.status(200).body(mock_devices);
        });
        let mut client = Client::new("password");
        client.ip = server.address().to_string();
        client.cookies.push("session=mocked_session_id".to_string());
        client.context_id = Some("test-context-id".to_string());
        let devices: Vec<Device> = client.get_devices().await;
        assert_eq!(devices[0].key, "test");
    }

    #[tokio::test]
    async fn test_get_metrics() {
        let server = MockServer::start();
        let mock_metrics = get_mock_metrics();
        let _m = server.mock(|when, then| {
            when.method(POST)
                .path("/ws")
                .header("x-context", "test-context-id");
            then.status(200).body(mock_metrics);
        });
        let mut client = Client::new("password");
        client.ip = server.address().to_string();
        client.cookies.push("session=mocked_session_id".to_string());
        client.context_id = Some("test-context-id".to_string());
        let metrics: Vec<Metrics> = client.get_metrics().await;
        assert_eq!(metrics[0].status["test"].traffic[0].timestamp, 1711483314);
    }

    #[tokio::test]
    async fn test_logout() {
        let server = MockServer::start();
        let _m = server.mock(|when, then| {
            when.method(POST)
                .path("/ws")
                .header("authorization", "X-Sah-Logout test-context-id")
                .header("cookie", "session=mocked_session_id");
            then.status(200);
        });
        let mut client = Client::new("password");
        client.ip = server.address().to_string();
        client.cookies.push("session=mocked_session_id".to_string());
        client.context_id = Some("test-context-id".to_string());
        client.logout().await;
        assert!(client.cookies.is_empty());
        assert!(client.context_id.is_none());
    }

    #[tokio::test]
    #[should_panic]
    async fn test_login_failure() {
        let server = MockServer::start();
        let _m = server.mock(|when, then| {
            when.method(POST)
                .path("/ws")
                .header("content-type", "application/x-sah-ws-4-call+json")
                .header("authorization", "X-Sah-Login");
            then.status(401);
        });
        let mut client = Client::new("password");
        client.ip = server.address().to_string();
        client.login().await;
    }

    #[tokio::test]
    #[should_panic]
    async fn test_authenticated_request_failure() {
        let server = MockServer::start();
        let _m = server.mock(|when, then| {
            when.method(POST)
                .path("/ws")
                .header("x-context", "test-context-id");
            then.status(500).body("Internal Server Error");
        });
        let mut client = Client::new("password");
        client.ip = server.address().to_string();
        client.cookies.push("session=mocked_session_id".to_string());
        client.context_id = Some("test-context-id".to_string());
        client.get_status().await;
    }
}
