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
        frame.render_widget(self, frame.size());
    }

    /// updates the application's state based on user input
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
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
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

        let BLOCK_HEIGHT = 10;
        let BLOCK_WIDTH = 40;

        let columns = inner_area.width / BLOCK_WIDTH;
        let column_width = inner_area.width / columns;
        let mut blocks = Vec::new();

        blocks.push(render_network_info(&inner_area));
        blocks.push(render_internet_info(&inner_area));
        blocks.push(render_dhcp_info(&inner_area));
        blocks.push(render_dns_info(&inner_area));
        blocks.push(render_traceroute_info(&inner_area));
        blocks.push(render_tcp_info(&inner_area));
        blocks.push(render_http_info(&inner_area));
        blocks.push(render_https_info(&inner_area));
        blocks.push(render_udp_info(&inner_area));
        blocks.push(render_ntp_info(&inner_area));
        blocks.push(render_quic_info(&inner_area));

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(area.height - 2), Constraint::Length(2)].as_ref())
            .split(inner_area);

        let rows = chunks[0];
        let columns_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints::<Vec<Constraint>>(
                (0..columns)
                    .map(|_| Constraint::Length(column_width))
                    .collect::<Vec<_>>(),
            )
            .split(rows);

        let mut widgets = Vec::new();
        for (i, block) in blocks.into_iter().enumerate() {
            let col = i % columns as usize;
            let row = i / columns as usize;
            if row < rows.height as usize {
                let rect = Rect::new(
                    columns_layout[col].x,
                    columns_layout[col].y + row as u16 * BLOCK_HEIGHT, // Assuming each block is 10 lines high
                    column_width,
                    BLOCK_HEIGHT,
                );
                widgets.push((block, rect));
            }
        }

        for (widget, rect) in widgets {
            widget.render(rect, buf);
        }
    }
}

fn render_network_info(area: &Rect) -> Block {
    let block = Block::default()
        .title("Network Info")
        .borders(Borders::ALL)
        .border_set(border::THICK);

    
}

fn render_internet_info(area: &Rect) -> Block {
    Block::default()
        .title("Internet Info")
        .borders(Borders::ALL)
}

fn render_dhcp_info(area: &Rect) -> Block {
    Block::default().title("DHCP Info").borders(Borders::ALL)
}

fn render_dns_info(area: &Rect) -> Block {
    Block::default().title("DNS Info").borders(Borders::ALL)
}

fn render_traceroute_info(area: &Rect) -> Block {
    Block::default()
        .title("Traceroute Info")
        .borders(Borders::ALL)
}

fn render_tcp_info(area: &Rect) -> Block {
    Block::default().title("TCP Info").borders(Borders::ALL)
}

fn render_http_info(area: &Rect) -> Block {
    Block::default().title("HTTP Info").borders(Borders::ALL)
}

fn render_https_info(area: &Rect) -> Block {
    Block::default().title("HTTPS Info").borders(Borders::ALL)
}

fn render_udp_info(area: &Rect) -> Block {
    Block::default().title("UDP Info").borders(Borders::ALL)
}

fn render_ntp_info(area: &Rect) -> Block {
    Block::default().title("NTP Info").borders(Borders::ALL)
}

fn render_quic_info(area: &Rect) -> Block {
    Block::default().title("QUIC Info").borders(Borders::ALL)
}
