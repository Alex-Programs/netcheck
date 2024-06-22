pub enum FetchedDataMessage {
    LocalInfo(LocalInfo),
    InternetInfo(InternetInfo),
    DHCPInfo(DHCPInfo),
    DNSInfo(DNSInfo),
    Traceroute(Traceroute),
    TCPInfo(TCPInfo),
    HTTPInfo(HTTPInfo),
    HTTPSInfo(HTTPSInfo),
    UDPInfo(UDPInfo),
    NTPInfo(NTPInfo),
    QUICInfo(QUICInfo),
}

#[derive(Debug, Default)]
pub struct NetworkInfo {
    pub local_info: LocalInfo,
    pub internet_info: InternetInfo,
    pub dhcp_info: DHCPInfo,
    pub dns_info: DNSInfo,
    pub traceroute: Traceroute,
    pub tcp_info: TCPInfo,
    pub http_info: HTTPInfo,
    pub https_info: HTTPSInfo,
    pub udp_info: UDPInfo,
    pub ntp_info: NTPInfo,
    pub quic_info: QUICInfo,
}

#[derive(Debug, Default)]
pub struct LocalInfo {
    pub local_ip: Option<String>,
    pub subnet_mask: Option<String>,
    pub gateway: Option<String>,
}

#[derive(Debug, Default)]
pub struct InternetInfo {
    pub public_ip: Option<String>,
    pub asn: Option<u32>,
    pub reverse_dns: Option<String>,
    pub isp: Option<String>,
    pub location: Option<String>,
    pub cloudflare_ping: Option<f64>,
}

#[derive(Debug, Default)]
pub struct DHCPInfo {
    pub dhcp_server: Option<String>,
    pub lease_time: Option<u64>,
    pub last_renewed: Option<u64>,
    pub dhcp_declared_dns: Option<Vec<String>>,
}

#[derive(Debug, Default)]
pub struct DNSInfo {
    pub primary_dns: Option<String>,
    pub can_access_primary: Option<bool>,
    pub secondary_dns: Option<String>,
    pub can_access_secondary: Option<bool>,
    pub tertiary_dns: Option<String>,
    pub can_access_tertiary: Option<bool>,
}

#[derive(Debug, Default)]
pub struct Traceroute {
    pub hops: Vec<TracerouteHop>,
}

#[derive(Debug)]
pub struct TracerouteHop {
    pub hop_number: u8,
    pub ip: String,
    pub latency: f64,
    pub jitter: f64,
    pub location: Option<String>,
}

#[derive(Debug, Default)]
pub struct TCPInfo {
    pub attempted_to_talk_on_list: Vec<(u16, bool)>,
}

#[derive(Debug, Default)]
pub struct HTTPInfo {
    pub can_access_1111: Option<bool>,
    pub can_access_google: Option<bool>,
    pub captive_portal: Option<bool>,
}

#[derive(Debug, Default)]
pub struct HTTPSInfo {
    pub can_access_1111: Option<bool>,
    pub can_access_google: Option<bool>,
    pub mitm_detected: Option<bool>,
}

#[derive(Debug, Default)]
pub struct UDPInfo {
    pub attempted_to_talk_on_list: Vec<(u16, bool)>,
}

#[derive(Debug, Default)]
pub struct NTPInfo {
    pub do_use_ntp: Option<bool>,
    pub ntp_server: Option<String>,
    pub can_access_ntp: Option<bool>,
    pub local_time: Option<u64>,
    pub server_time: Option<u64>,
}

#[derive(Debug, Default)]
pub struct QUICInfo {
    pub can_access_1111: Option<bool>,
    pub can_access_google: Option<bool>,
}