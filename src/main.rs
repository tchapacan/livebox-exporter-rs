mod livebox_client_rs;

use clap::{value_parser, Arg, ArgAction, ArgMatches, Command};
use hyper::{Body, Request};
use livebox_client_rs::{
    client::Client,
    devices::Device,
    metrics::{Metrics, TrafficData},
    status::Status,
    wan::WANConfiguration,
};
use log::{trace, LevelFilter};
use prometheus_exporter_base::prelude::*;
use std::{
    env,
    error::Error,
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

#[derive(Debug, Clone, Default)]
struct MyOptions {}

static LIVEBOX_EXPORTER_NAME: &str = env!("CARGO_PKG_NAME");
static LIVEBOX_EXPORTER_VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() {
    let matches = Command::new(LIVEBOX_EXPORTER_NAME)
        .version(LIVEBOX_EXPORTER_VERSION)
        .author("tchapacan")
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .help("exporter port")
                .value_parser(value_parser!(u16))
                .default_value("9100"),
        )
        .arg(
            Arg::new("address")
                .short('l')
                .long("listen")
                .help("listen address")
                .value_parser(value_parser!(String))
                .default_value("0.0.0.0"),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("verbose logging")
                .action(ArgAction::Count),
        )
        .arg(
            Arg::new("password")
                .short('P')
                .long("password")
                .help("Livebox password [required]")
                .value_parser(value_parser!(String))
                .required(true),
        )
        .arg(
            Arg::new("gateway")
                .short('G')
                .long("gateway")
                .help("Livebox gateway ip address")
                .value_parser(value_parser!(String))
                .default_value("192.168.1.1"),
        )
        .get_matches();

    let verbosity = matches.get_count("verbose");

    let log_level = match verbosity {
        0 => LevelFilter::Off,
        1 => LevelFilter::Info,
        2 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };

    env::set_var(
        "RUST_LOG",
        format!(
            "folder_size={},livebox-exporter-rs={}",
            log_level, log_level
        ),
    );
    env_logger::Builder::new().filter_level(log_level).init();

    let bind: u16 = *matches.get_one("port").unwrap();
    let listening_address = match matches.get_one::<String>("address") {
        Some(password) => password.clone(),
        None => {
            eprintln!("Please provide a listening address with the -l or --listen flag.");
            std::process::exit(1);
        }
    };
    let ip_addr: IpAddr = listening_address.parse().expect("Invalid IP address");
    let addr: SocketAddr = SocketAddr::new(ip_addr, bind);
    let server_options = ServerOptions {
        addr,
        authorization: Authorization::None,
    };
    println!("Starting exporter with options {:?}", addr);
    render_prometheus(server_options, MyOptions::default(), |request, options| {
        render_livebox_metrics(request, options, matches)
    })
    .await;
}

async fn render_livebox_metrics(
    request: Request<Body>,
    _options: Arc<MyOptions>,
    matches: ArgMatches,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    trace!(
        "In our render_prometheus(request == {:?}, options == {:?})",
        request,
        _options
    );
    let livebox_password = match matches.get_one::<String>("password") {
        Some(password) => password.clone(),
        None => {
            eprintln!("Please provide a livebox password with the -P or --password flag.");
            std::process::exit(1);
        }
    };
    let ip = match matches.get_one::<String>("gateway") {
        Some(gateway) => gateway.clone(),
        None => {
            eprintln!("Please provide a livebox gateway ip address with the -G or --gateway flag.");
            std::process::exit(1);
        }
    };
    let mut client = Client::new(&livebox_password, &ip);
    client.login().await;
    let status = client.get_status().await;
    let wan = client.get_wan_config().await;
    let metrics = client.get_metrics().await;
    let devices = client.get_devices().await;
    let rendered_metrics = vec![
        render_livebox_info_metric(
            &status,
            "livebox_infos_status",
            "Livebox general status",
            |s| if s.device_status == "Up" { 1 } else { 0 },
        ),
        render_livebox_info_metric(&status, "livebox_infos_uptime", "Livebox uptime", |s| {
            s.up_time.try_into().unwrap()
        }),
        render_livebox_info_metric(
            &status,
            "livebox_infos_reboot",
            "Livebox count of reboots",
            |s| s.number_of_reboots.try_into().unwrap(),
        ),
        render_livebox_status_metric(&wan, "livebox_wan_status", "wan"),
        render_livebox_status_metric(&wan, "livebox_link_status", "link"),
        render_livebox_interface_metric(
            &metrics,
            "livebox_interface_bytes_rx",
            "Livebox interface bytes RX",
            |e: &TrafficData| e.rx_counter.try_into().unwrap(),
            "rx",
        ),
        render_livebox_interface_metric(
            &metrics,
            "livebox_interface_bytes_tx",
            "Livebox interface bytes TX",
            |e: &TrafficData| e.tx_counter.try_into().unwrap(),
            "tx",
        ),
        render_livebox_devices_metric(
            &devices,
            "livebox_device_status",
            "Livebox connected devices status",
            |d| if d.active { 1 } else { 0 },
        ),
    ];

    client.logout().await;
    Ok(rendered_metrics.join(""))
}

fn create_metric<'a>(name: &'a str, help: &'a str) -> PrometheusMetric<'a> {
    PrometheusMetric::build()
        .with_name(name)
        .with_metric_type(MetricType::Gauge)
        .with_help(help)
        .build()
}

fn render_livebox_info_metric<F>(status: &Status, name: &str, help: &str, value_fn: F) -> String
where
    F: FnOnce(&Status) -> usize,
{
    create_metric(name, help)
        .render_and_append_instance(
            &PrometheusInstance::new()
                .with_label("hardware", "livebox")
                .with_label("manufacturer", &*status.manufacturer)
                .with_label("manufacturer_oui", &*status.manufacturer_oui)
                .with_label("model_name", &*status.model_name)
                .with_label("product_class", &*status.product_class)
                .with_label("serial_number", &*status.serial_number)
                .with_label("hardware_version", &*status.hardware_version)
                .with_label("software_version", &*status.software_version)
                .with_label("country", &*status.country)
                .with_label("external_ip_address", &*status.external_ip_address)
                .with_label("base_mac", &*status.base_mac)
                .with_value(value_fn(status))
                .with_current_timestamp()
                .expect("Error getting the current UNIX epoch"),
        )
        .render()
}

fn render_livebox_status_metric(wan_config: &WANConfiguration, name: &str, port: &str) -> String {
    create_metric(name, &format!("Livebox {} status", port))
        .render_and_append_instance(
            &PrometheusInstance::new()
                .with_label("port", port)
                .with_label("link_type", &*wan_config.link_type)
                .with_label("protocol", &*wan_config.protocol)
                .with_label("mac_address", &*wan_config.mac_address)
                .with_label("ip_address", &*wan_config.ip_address)
                .with_label("remote_gateway", &*wan_config.remote_gateway)
                .with_label("remote_gadns_serversteway", &*wan_config.dns_servers)
                .with_label("ipv6_address", &*wan_config.ipv6_address)
                .with_value(if port == "wan" {
                    wan_config.wan_state == "up"
                } else {
                    wan_config.link_state == "up"
                } as usize)
                .with_current_timestamp()
                .expect("Error getting the current UNIX epoch"),
        )
        .render()
}

fn render_livebox_interface_metric<F>(
    metrics: &[Metrics],
    name: &str,
    help: &str,
    value_fn: F,
    direction: &str,
) -> String
where
    F: Fn(&TrafficData) -> usize,
{
    let mut rendered_metrics = create_metric(name, help);
    for metric in metrics {
        for (interface_name, interface_data) in metric.status.iter() {
            for entry in &interface_data.traffic {
                rendered_metrics.render_and_append_instance(
                    &PrometheusInstance::new()
                        .with_label("interface_name", &*interface_name.clone())
                        .with_label("direction", direction)
                        .with_value(value_fn(entry))
                        .with_current_timestamp()
                        .expect("Error getting the current UNIX epoch"),
                );
            }
        }
    }
    rendered_metrics.render()
}

fn render_livebox_devices_metric<F>(
    devices: &[Device],
    name: &str,
    help: &str,
    value_fn: F,
) -> String
where
    F: Fn(&Device) -> usize,
{
    let mut rendered_metrics = create_metric(name, help);
    for device in devices {
        rendered_metrics.render_and_append_instance(
            &PrometheusInstance::new()
                .with_label("device_name", &*device.name)
                .with_label("device_type", &*device.device_type)
                .with_label("discovery_source", &*device.discovery_source)
                .with_label(
                    "ip_address",
                    &*device.ip_address.clone().unwrap_or("".to_string()),
                )
                .with_value(value_fn(device))
                .with_current_timestamp()
                .expect("Error getting the current UNIX epoch"),
        );
    }
    rendered_metrics.render()
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::livebox_client_rs::metrics::DeviceMetrics;
    use maplit::hashmap;

    fn parse_args(args: Vec<&str>) -> clap::ArgMatches {
        Command::new(LIVEBOX_EXPORTER_NAME)
            .version(LIVEBOX_EXPORTER_VERSION)
            .author("tchapacan")
            .arg(
                Arg::new("port")
                    .short('p')
                    .long("port")
                    .help("exporter port")
                    .value_name("PORT")
                    .default_value("9100"),
            )
            .arg(
                Arg::new("address")
                    .short('l')
                    .long("listen")
                    .help("listen address")
                    .value_name("ADDRESS")
                    .default_value("0.0.0.0"),
            )
            .arg(
                Arg::new("verbose")
                    .short('v')
                    .long("verbose")
                    .help("verbose logging")
                    .action(ArgAction::Count),
            )
            .arg(
                Arg::new("password")
                    .short('P')
                    .long("password")
                    .help("Livebox password")
                    .value_name("PASSWORD")
                    .required(true),
            )
            .arg(
                Arg::new("gateway")
                    .short('G')
                    .long("gateway")
                    .help("Livebox gateway ip address")
                    .value_parser(value_parser!(String))
                    .default_value("192.168.1.1"),
            )
            .get_matches_from(args)
    }

    #[test]
    fn test_parse_args_default() {
        let args = vec!["livebox-exporter-rs", "-P", "mypassword"];
        let matches = parse_args(args);
        assert_eq!(
            matches.get_one::<String>("port"),
            Some(&String::from("9100"))
        );
        assert_eq!(
            matches.get_one::<String>("address"),
            Some(&String::from("0.0.0.0"))
        );
        assert_eq!(matches.get_count("verbose"), 0);
        assert_eq!(
            matches.get_one::<String>("password"),
            Some(&String::from("mypassword"))
        );
        assert_eq!(
            matches.get_one::<String>("gateway"),
            Some(&String::from("192.168.1.1"))
        );
    }

    #[test]
    fn test_parse_args_custom() {
        let args = vec![
            "livebox-exporter-rs",
            "-p",
            "1234",
            "--listen",
            "127.0.0.1",
            "-vvv",
            "-P",
            "mypassword",
            "-G",
            "192.168.1.10",
        ];
        let matches = parse_args(args);
        assert_eq!(
            matches.get_one::<String>("port"),
            Some(&String::from("1234"))
        );
        assert_eq!(
            matches.get_one::<String>("address"),
            Some(&String::from("127.0.0.1"))
        );
        assert_eq!(matches.get_count("verbose"), 3);
        assert_eq!(
            matches.get_one::<String>("password"),
            Some(&String::from("mypassword"))
        );
        assert_eq!(
            matches.get_one::<String>("gateway"),
            Some(&String::from("192.168.1.10"))
        );
    }

    #[test]
    fn test_render_livebox_info_metric() {
        let status = Status {
            device_status: "Up".to_string(),
            up_time: 12345,
            number_of_reboots: 10,
            manufacturer: "test".to_string(),
            manufacturer_oui: "test".to_string(),
            model_name: "test".to_string(),
            description: "test".to_string(),
            product_class: "test".to_string(),
            serial_number: "test".to_string(),
            hardware_version: "test".to_string(),
            software_version: "test".to_string(),
            rescue_version: "test".to_string(),
            modem_firmware_version: "test".to_string(),
            enabled_options: "test".to_string(),
            additional_hardware_version: "test".to_string(),
            additional_software_version: "test".to_string(),
            spec_version: "test".to_string(),
            provisioning_code: "test".to_string(),
            first_use_date: "test".to_string(),
            device_log: "test".to_string(),
            vendor_config_file_number_of_entries: 1,
            manufacturer_url: "test".to_string(),
            country: "test".to_string(),
            external_ip_address: "test".to_string(),
            upgrade_occurred: true,
            reset_occurred: true,
            restore_occurred: true,
            standby_occurred: true,
            softathome_additional_software_versions: "test".to_string(),
            base_mac: "test".to_string(),
        };
        let expected_output = "# HELP test_name test_help\n# TYPE test_name gauge\ntest_name{hardware=\"livebox\",manufacturer=\"test\",manufacturer_oui=\"test\",model_name=\"test\",product_class=\"test\",serial_number=\"test\",hardware_version=\"test\",software_version=\"test\",country=\"test\",external_ip_address=\"test\",base_mac=\"test\"} 1 TIMESTAMP_PLACEHOLDER\n";
        let result = render_livebox_info_metric(&status, "test_name", "test_help", |s| {
            if s.device_status == "Up" {
                1
            } else {
                0
            }
        });
        let expected_output_with_timestamp = expected_output.replace(
            "TIMESTAMP_PLACEHOLDER",
            &result.split_whitespace().last().unwrap(),
        );
        assert_eq!(result, expected_output_with_timestamp);
    }

    #[test]
    fn test_render_livebox_status_metric() {
        let wan = WANConfiguration {
            wan_state: "up".to_string(),
            link_type: "test".to_string(),
            link_state: "up".to_string(),
            mac_address: "test".to_string(),
            protocol: "test".to_string(),
            connection_state: "test".to_string(),
            last_connection_error: "test".to_string(),
            ip_address: "test".to_string(),
            remote_gateway: "test".to_string(),
            dns_servers: "test".to_string(),
            ipv6_address: "test".to_string(),
            ipv6_delegated_prefix: "test".to_string(),
        };
        let expected_output = "# HELP test_name Livebox wan status\n# TYPE test_name gauge\ntest_name{port=\"wan\",link_type=\"test\",protocol=\"test\",mac_address=\"test\",ip_address=\"test\",remote_gateway=\"test\",remote_gadns_serversteway=\"test\",ipv6_address=\"test\"} 1 TIMESTAMP_PLACEHOLDER\n";
        let result = render_livebox_status_metric(&wan, "test_name", "wan");
        let expected_output_with_timestamp = expected_output.replace(
            "TIMESTAMP_PLACEHOLDER",
            &result.split_whitespace().last().unwrap(),
        );
        assert_eq!(result, expected_output_with_timestamp);
    }

    #[test]
    fn test_render_livebox_interface_metric() {
        let metrics = vec![Metrics {
            status: hashmap! {
                "test_interface".to_string() => DeviceMetrics {
                    traffic: vec![
                        TrafficData {
                            rx_counter: 123,
                            tx_counter: 456,
                            timestamp: 789,
                        },
                    ],
                },
            },
        }];
        let expected_output = "# HELP test_name test_help\n# TYPE test_name gauge\ntest_name{interface_name=\"test_interface\",direction=\"rx\"} 123 TIMESTAMP_PLACEHOLDER\n";
        let result = render_livebox_interface_metric(
            &metrics,
            "test_name",
            "test_help",
            |e: &TrafficData| e.rx_counter.try_into().unwrap(),
            "rx",
        );
        let expected_output_with_timestamp = expected_output.replace(
            "TIMESTAMP_PLACEHOLDER",
            &result.split_whitespace().last().unwrap(),
        );
        assert_eq!(result, expected_output_with_timestamp);
    }

    #[test]
    fn test_render_livebox_devices_metric() {
        let devices = vec![Device {
            key: "test".to_string(),
            name: "test".to_string(),
            discovery_source: "test".to_string(),
            active: true,
            device_type: "test".to_string(),
            tags: "test".to_string(),
            ip_address: Some("test".to_string()),
            ssid: Some("test".to_string()),
            channel: Some(1),
        }];
        let expected_output = "# HELP test_name test_help\n# TYPE test_name gauge\ntest_name{device_name=\"test\",device_type=\"test\",discovery_source=\"test\",ip_address=\"test\"} 1 TIMESTAMP_PLACEHOLDER\n";
        let result = render_livebox_devices_metric(&devices, "test_name", "test_help", |d| {
            if d.active {
                1
            } else {
                0
            }
        });
        let expected_output_with_timestamp = expected_output.replace(
            "TIMESTAMP_PLACEHOLDER",
            &result.split_whitespace().last().unwrap(),
        );
        assert_eq!(result, expected_output_with_timestamp);
    }

    // TODO : WIP More to come..
}
