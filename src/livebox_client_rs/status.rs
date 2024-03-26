use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
#[serde(rename_all="PascalCase")]
pub struct Status {
    pub manufacturer: String,
    #[serde(rename(deserialize = "ManufacturerOUI"))]
    pub manufacturer_oui: String,
    pub model_name: String,
    pub description: String,
    pub product_class: String,
    pub serial_number: String,
    pub hardware_version: String,
    pub software_version: String,
    pub rescue_version: String,
    pub modem_firmware_version: String,
    pub enabled_options: String,
    pub additional_hardware_version: String,
    pub additional_software_version: String,
    pub spec_version: String,
    pub provisioning_code: String,
    pub up_time: u32,
    pub first_use_date: String,
    pub device_log: String,
    pub vendor_config_file_number_of_entries: u32,
    #[serde(rename(deserialize = "ManufacturerURL"))]
    pub manufacturer_url: String,
    pub country: String,
    #[serde(rename(deserialize = "ExternalIPAddress"))]
    pub external_ip_address: String,
    pub device_status: String,
    pub number_of_reboots: u32,
    pub upgrade_occurred: bool,
    pub reset_occurred: bool,
    pub restore_occurred: bool,
    pub standby_occurred: bool,
    #[serde(rename(deserialize = "X_SOFTATHOME-COM_AdditionalSoftwareVersions"))]
    pub softathome_additional_software_versions: String,
    #[serde(rename(deserialize = "BaseMAC"))]
    pub base_mac: String,
}
