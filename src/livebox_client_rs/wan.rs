use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct WANConfiguration {
    pub wan_state: String,
    pub link_type: String,
    pub link_state: String,
    #[serde(rename(deserialize = "MACAddress"))]
    pub mac_address: String,
    pub protocol: String,
    pub connection_state: String,
    pub last_connection_error: String,
    #[serde(rename(deserialize = "IPAddress"))]
    pub ip_address: String,
    pub remote_gateway: String,
    #[serde(rename(deserialize = "DNSServers"))]
    pub dns_servers: String,
    #[serde(rename(deserialize = "IPv6Address"))]
    pub ipv6_address: String,
    #[serde(rename(deserialize = "IPv6DelegatedPrefix"))]
    pub ipv6_delegated_prefix: String,
}
