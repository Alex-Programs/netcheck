use color_eyre::{
    eyre::{bail, WrapErr},
    Result,
};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    prelude::*,
    symbols::border,
    widgets::{
        block::{Position, Title},
        *,
    },
};
use std::net::IpAddr;

mod errors;
mod tui;

fn main() -> Result<()> {
    errors::install_hooks()?;
    let mut terminal = tui::init()?;
    App::default().run(&mut terminal)?;
    tui::restore()?;
    Ok(())
}

#[derive(Debug, Default)]
pub struct App {
    exit: bool,
    network_info: NetworkInfo,
    throbber_state: throbber_widgets_tui::ThrobberState,
}

#[derive(Debug, Default)]
pub struct NetworkInfo {
    local_info: LocalInfo,
    internet_info: InternetInfo,
    speed_info: SpeedInfo,
    dhcp_info: DHCPInfo,
    dns_info: DNSInfo,
    traceroute: Traceroute,
    tcp_info: TCPInfo,
    http_info: HTTPInfo,
    https_info: HTTPSInfo,
    udp_info: UDPInfo,
    ntp_info: NTPInfo,
    quic_info: QUICInfo,
}

#[derive(Debug, Default)]
pub struct LocalInfo {
    local_ip: Option<IpAddr>,
    subnet_mask: Option<IpAddr>,
    gateway: Option<IpAddr>,
}

#[derive(Debug, Default)]
pub struct InternetInfo {
    public_ip: Option<IpAddr>,
    asn: Option<u32>,
    reverse_dns: Option<String>,
    isp: Option<String>,
    location: Option<String>,
}

#[derive(Debug, Default)]
pub struct SpeedInfo {
    download_speed: Option<f64>,
    upload_speed: Option<f64>,
}

#[derive(Debug, Default)]
pub struct DHCPInfo {
    dhcp_server: Option<IpAddr>,
    lease_time: Option<u32>,
    last_renewed: Option<u32>,
    dhcp_declared_dns: Option<Vec<IpAddr>>,
}

#[derive(Debug, Default)]
pub struct DNSInfo {
    primary_dns: Option<IpAddr>,
    can_access_primary: Option<bool>,
    secondary_dns: Option<IpAddr>,
    can_access_secondary: Option<bool>,
    tertiary_dns: Option<IpAddr>,
    can_access_tertiary: Option<bool>,
    dhcp_declared_dns: Option<Vec<IpAddr>>,
    can_access_dhcp_dns: Option<bool>,
}

#[derive(Debug, Default)]
pub struct Traceroute {
    hops: Vec<TracerouteHop>,
}

#[derive(Debug)]
pub struct TracerouteHop {
    hop_number: u8,
    ip: IpAddr,
    latency: f64,
    jitter: f64,
    location: Option<String>,
}

#[derive(Debug, Default)]
pub struct TCPInfo {
    attempted_to_talk_on_list: Vec<(u16, bool)>,
}

#[derive(Debug, Default)]
pub struct HTTPInfo {
    can_access_1111: Option<bool>,
    can_access_google: Option<bool>,
}

#[derive(Debug, Default)]
pub struct HTTPSInfo {
    can_access_1111: Option<bool>,
    can_access_google: Option<bool>,
}

#[derive(Debug, Default)]
pub struct UDPInfo {
    attempted_to_talk_on_list: Vec<(u16, bool)>,
}

#[derive(Debug, Default)]
pub struct NTPInfo {
    do_use_ntp: Option<bool>,
    ntp_server: Option<IpAddr>,
    can_access_ntp: Option<bool>,
    local_time: Option<u64>,
    server_time: Option<u64>,
}

#[derive(Debug, Default)]
pub struct QUICInfo {
    can_access_1111: Option<bool>,
    can_access_google: Option<bool>,
}

impl App {
    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut tui::Tui) -> Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.render_frame(frame))?;
            self.handle_events().wrap_err("handle events failed")?;

            // Update the throbber state
            self.throbber_state.calc_next();

            // Sleep for 50ms to avoid hogging the CPU
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        Ok(())
    }

    fn render_frame(&self, frame: &mut Frame) {
        let area = frame.size();
        let buf = frame.buffer_mut();

        let title = Title::from(" NETCHECK ".bold());
        let instructions = Title::from(Line::from(vec![
            " Quit ".into(),
            "<Q> ".blue().bold(),
        ]));
        let exterior_block = Block::default()
            .title(title.alignment(Alignment::Center))
            .title(
                instructions
                    .alignment(Alignment::Center)
                    .position(Position::Bottom),
            )
            .borders(Borders::TOP)
            .border_set(border::THICK);

        let inner_area = exterior_block.inner(area);
        exterior_block.render(area, buf);

        let BLOCK_HEIGHT = 5;
        let BLOCK_WIDTH = 40;

        let columns = inner_area.width / BLOCK_WIDTH;
        let column_width = inner_area.width / columns;
        let mut blocks = Vec::new();

        blocks.push(self.render_network_info(inner_area));
        blocks.push(self.render_internet_info(inner_area));
        blocks.push(self.render_dhcp_info(inner_area));
        blocks.push(self.render_dns_info(inner_area));
        blocks.push(self.render_traceroute_info(inner_area));
        blocks.push(self.render_tcp_info(inner_area));
        blocks.push(self.render_http_info(inner_area));
        blocks.push(self.render_https_info(inner_area));
        blocks.push(self.render_udp_info(inner_area));
        blocks.push(self.render_ntp_info(inner_area));
        blocks.push(self.render_quic_info(inner_area));

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(area.height - 2), Constraint::Length(2)].as_ref())
            .split(inner_area);

        let rows = chunks[0];
        let columns_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                (0..columns)
                    .map(|_| Constraint::Length(column_width))
                    .collect::<Vec<Constraint>>(),
            )
            .split(rows);

        for (i, block) in blocks.into_iter().enumerate() {
            let col = i % columns as usize;
            let row = i / columns as usize;
            let y_position = columns_layout[col].y + row as u16 * BLOCK_HEIGHT;

            // Ensure the block is within the terminal area
            if y_position + BLOCK_HEIGHT <= area.height {
                let rect = Rect::new(
                    columns_layout[col].x,
                    y_position,
                    column_width,
                    BLOCK_HEIGHT,
                );
                block.render(rect, buf); // Render each block directly
            }
        }
    }

    fn handle_events(&mut self) -> Result<()> {
        match event::read()? {
            // it's important to check that the event is a key press event as
            // crossterm also emits key release and repeat events on Windows.
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => self
                .handle_key_event(key_event)
                .wrap_err_with(|| format!("handling key event failed:\n{key_event:#?}")),
            _ => Ok(()),
        }
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<()> {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Char('Q') => self.exit(),
            _ => {}
        }
        Ok(())
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn render_network_info(&self, _area: Rect) -> Paragraph {
        let text = vec![
            Line::from("Local IP: 192.168.0.1"),
            Line::from("Subnet Mask: 255.255.255.0"),
            Line::from("Gateway: 192.168.0.254"),
        ];
        Paragraph::new(Text::from(text))
            .block(Block::default().title("Network Info").borders(Borders::ALL))
    }

    fn render_internet_info(&self, _area: Rect) -> Paragraph {
        let text = vec![
            Line::from("Public IP: 203.0.113.1"),
            Line::from("ASN: 12345"),
            Line::from("Reverse DNS: example.com"),
            Line::from("ISP: Example ISP"),
            Line::from("Location: Somewhere, Earth"),
        ];
        Paragraph::new(Text::from(text))
            .block(Block::default().title("Internet Info").borders(Borders::ALL))
    }

    fn render_dhcp_info(&self, _area: Rect) -> Paragraph {
        let text = vec![
            Line::from("DHCP Server: 192.168.0.1"),
            Line::from("Lease Time: 86400"),
            Line::from("Last Renewed: 43200"),
        ];
        Paragraph::new(Text::from(text))
            .block(Block::default().title("DHCP Info").borders(Borders::ALL))
    }

    fn render_dns_info(&self, _area: Rect) -> Paragraph {
        let text = vec![
            Line::from("Primary DNS: 8.8.8.8"),
            Line::from("Can Access Primary: Yes"),
            Line::from("Secondary DNS: 8.8.4.4"),
            Line::from("Can Access Secondary: Yes"),
            Line::from("Tertiary DNS: 1.1.1.1"),
            Line::from("Can Access Tertiary: Yes"),
        ];
        Paragraph::new(Text::from(text))
            .block(Block::default().title("DNS Info").borders(Borders::ALL))
    }

    fn render_traceroute_info(&self, _area: Rect) -> Paragraph {
        let text = vec![
            Line::from("Hop 1: 192.168.0.1 - Latency: 1ms"),
            Line::from("Hop 2: 203.0.113.1 - Latency: 10ms"),
            Line::from("Hop 3: 198.51.100.1 - Latency: 20ms"),
        ];
        Paragraph::new(Text::from(text))
            .block(Block::default().title("Traceroute Info").borders(Borders::ALL))
    }

    fn render_tcp_info(&self, _area: Rect) -> Paragraph {
        let text = vec![
            Line::from("Port 80: Success"),
            Line::from("Port 443: Success"),
            Line::from("Port 8080: Fail"),
        ];
        Paragraph::new(Text::from(text))
            .block(Block::default().title("TCP Info").borders(Borders::ALL))
    }

    fn render_http_info(&self, _area: Rect) -> Paragraph {
        let text = vec![
            Line::from("Can Access 1.1.1.1: Yes"),
            Line::from("Can Access Google: Yes"),
        ];
        Paragraph::new(Text::from(text))
            .block(Block::default().title("HTTP Info").borders(Borders::ALL))
    }

    fn render_https_info(&self, _area: Rect) -> Paragraph {
        let text = vec![
            Line::from("Can Access 1.1.1.1: Yes"),
            Line::from("Can Access Google: Yes"),
        ];
        Paragraph::new(Text::from(text))
            .block(Block::default().title("HTTPS Info").borders(Borders::ALL))
    }

    fn render_udp_info(&self, _area: Rect) -> Paragraph {
        let text = vec![
            Line::from("Port 53: Success"),
            Line::from("Port 123: Success"),
        ];
        Paragraph::new(Text::from(text))
            .block(Block::default().title("UDP Info").borders(Borders::ALL))
    }

    fn render_ntp_info(&self, _area: Rect) -> Paragraph {
        let text = vec![
            Line::from("Use NTP: Yes"),
            Line::from("NTP Server: 129.6.15.28"),
            Line::from("Can Access NTP: Yes"),
            Line::from("Local Time: 1627890123"),
            Line::from("Server Time: 1627890124"),
        ];
        Paragraph::new(Text::from(text))
            .block(Block::default().title("NTP Info").borders(Borders::ALL))
    }

    fn render_quic_info(&self, _area: Rect) -> Paragraph {
        let text = vec![
            Line::from("Can Access 1.1.1.1: Yes"),
            Line::from("Can Access Google: Yes"),
        ];
        Paragraph::new(Text::from(text))
            .block(Block::default().title("QUIC Info").borders(Borders::ALL))
    }
}
