
use esp_idf_svc::{eventloop::EspSystemEventLoop, hal::peripheral, handle::RawHandle, wifi::{ClientConfiguration, Configuration, EspWifi}};
use esp_idf_svc::netif::EspNetif;
use esp_idf_svc::sys::{
    esp_err_to_name, esp_netif_dns_info_t, esp_netif_set_dns_info, esp_netif_dns_type_t_ESP_NETIF_DNS_MAIN,
};

pub struct EventEmitters {
    pub connection_status: qdb::EventEmitter<bool>,
}

pub struct Worker {
    ssid: String,
    password: String,
    is_connected: bool,
    handle: Box<EspWifi<'static>>,
    pub dns: Option<String>,
    pub emitters: EventEmitters,
}

impl Worker {
    pub fn new(ssid: &str, password: &str, modem: impl peripheral::Peripheral<P = esp_idf_svc::hal::modem::Modem> + 'static, sysloop: EspSystemEventLoop) -> Self {
        Self {
            ssid: ssid.to_string(),
            password: password.to_string(),
            is_connected: false,
            handle: Box::new(EspWifi::new( modem, sysloop.clone(), None).unwrap()),
            dns: None,
            emitters: EventEmitters {
                connection_status: qdb::EventEmitter::new(),
            },
        }
    }
}

impl qdb::WorkerTrait for Worker {
    fn intialize(&mut self, ctx: qdb::ApplicationContext) -> qdb::Result<()> {
        ctx.logger().info( format!("[WiFiConnector::initialize] Initializing WiFi connector (SSID: '{}')", self.ssid).as_str());

        let conf = Configuration::Client(ClientConfiguration {
            ssid: self.ssid.as_str().try_into().expect("SSID too long"),
            password: self.password.as_str().try_into().expect("Password too long"),
            ..Default::default()
        });

        self.handle.set_configuration(&conf)?;

        self.handle.start()?;
        self.handle.connect()?;

        Ok(())
    }

    fn do_work(&mut self, ctx: qdb::ApplicationContext) -> qdb::Result<()> {
        if self.handle.is_connected()? {
            if !self.is_connected {
                ctx.logger().info( "[WiFiConnector::do_work] WiFi connected");
                self.is_connected = true;
                self.emitters.connection_status.emit(self.is_connected);

                if let Some(dns) = &self.dns {
                    set_dns_server(dns, self.handle.sta_netif())?;
                    ctx.logger().info(format!("Dns: {}", self.handle.sta_netif().get_dns().to_string()).as_str());
                }
            }
        } else {
            if self.is_connected {
                ctx.logger().info( "[WiFiConnector::do_work] WiFi disconnected");
                self.is_connected = false;
                self.emitters.connection_status.emit(self.is_connected);
            } 
        }
        Ok(())
    }

    fn deinitialize(&mut self, ctx: qdb::ApplicationContext) -> qdb::Result<()> {
        ctx.logger().info( "[WiFiConnector::deinitialize] Deinitializing WiFi connector");
        self.handle.disconnect()?;
        Ok(())
    }

    fn process_events(&mut self) -> qdb::Result<()> {
        Ok(())
    }
}

fn set_dns_server(dns_ip: &str, netif: &EspNetif) -> qdb::Result<()> {
    // Parse DNS server IP
    let dns_ip_addr = dns_ip.parse::<std::net::Ipv4Addr>()?;
    let mut dns_info = esp_netif_dns_info_t {
        ip: esp_idf_svc::sys::_ip_addr {
            u_addr: esp_idf_svc::sys::_ip_addr__bindgen_ty_1 {
                ip4: esp_idf_svc::sys::esp_ip4_addr {
                    addr: u32::from(dns_ip_addr).to_be(), // convert to network byte order
                },
            },
            type_: esp_idf_svc::sys::lwip_ip_addr_type_IPADDR_TYPE_V4 as u8,
        },
    };

    // Set the main DNS server
    let result = unsafe {
        esp_netif_set_dns_info(
            netif.handle(),
            esp_netif_dns_type_t_ESP_NETIF_DNS_MAIN,
            &mut dns_info as *mut esp_netif_dns_info_t,
        )
    };

    // Check for errors
    if result != 0 {
        return Err(format!(
            "Failed to set DNS server: {:?}",
            unsafe { esp_err_to_name(result) }
        ).into());
    }

    Ok(())
}

