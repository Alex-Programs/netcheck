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
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};

use std::thread;

mod errors;
mod tui;
mod netlib;
mod internal_comms;
use internal_comms::FetchedDataMessage;

mod fetch_local;
mod fetch_dns;

const BLOCK_HEIGHT: u16 = 10;
const BLOCK_WIDTH: u16 = 30;

fn main() -> Result<()> {
    errors::install_hooks()?;
    let mut terminal = tui::init()?;

    let mut app = App::default();

    // Get list of network interfaces
    let interface_list = netlib::get_interfaces();

    // If there are no interfaces, bail out
    if interface_list.is_empty() {
        bail!("No network interfaces found");
    }

    // If there is one, automatically select it
    if interface_list.len() == 1 {
        app.chosen_interface = Some(interface_list[0].clone());
        app.interface_list = interface_list;
        app.stage = ApplicationStage::Running;
    } else {
        app.interface_list = interface_list;
    }

    app.run(&mut terminal)?;
    tui::restore()?;
    Ok(())
}

#[derive(Debug)]
enum ApplicationStage {
    PickInterface,
    Running,
}

impl Default for ApplicationStage {
    fn default() -> Self {
        ApplicationStage::PickInterface
    }
}

#[derive(Debug, Default)]
pub struct App {
    exit: bool,
    network_info: internal_comms::NetworkInfo,
    stage: ApplicationStage,
    interface_list: Vec<String>,
    interface_hover_index: usize,
    chosen_interface: Option<String>,
    receive_new_data_channel: Option<mpsc::Receiver<FetchedDataMessage>>,
    block_width_practice: u32,
}

impl App {
    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut tui::Tui) -> Result<()> {
        while !self.exit {
            // Pull in any new data from the channel
            if let Some(ref receive_new_data_channel) = self.receive_new_data_channel {
                for message in receive_new_data_channel.try_iter() {
                    match message {
                        FetchedDataMessage::LocalInfo(local_info) => {
                            self.network_info.local_info = local_info;
                        }
                        FetchedDataMessage::DNSInfo(dns_info) => {
                            self.network_info.dns_info = dns_info;
                        }
                        _ => {}
                    }
                }
            }

            terminal.draw(|frame| self.render_frame(frame))?;

            self.handle_events().wrap_err("handle events failed")?;
        }
        Ok(())
    }

    fn render_frame(&mut self, frame: &mut Frame) {
        match self.stage {
            ApplicationStage::PickInterface => self.pick_interface_render_frame(frame),
            ApplicationStage::Running => self.running_render_frame(frame),
        }
    }

    fn pick_interface_render_frame(&self, frame: &mut Frame) {
        let area = frame.size();
        let buf = frame.buffer_mut();
    
        let title = Title::from(" NETCHECK ".bold());
    
        let instructions = Title::from(Line::from(vec![
            " Quit ".into(), "<Q> ".blue().bold(),
            " Up ".into(), "↑".blue().bold(),
            " Down ".into(), "↓".blue().bold(),
            " Select ".into(), "<Enter>".blue().bold(),
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
    
        // Insert a subtitle of "Pick an interface"
        let subtitle = Title::from(" Pick an interface ".bold());
    
        let interfaces_block = Block::default()
            .borders(Borders::ALL)
            .title_alignment(Alignment::Left)
            .title(subtitle);
    
        let interface_area = interfaces_block.inner(inner_area);
    
        let interface_items: Vec<ListItem> = self.interface_list.iter().enumerate().map(|(i, interface)| {
            let content = if self.interface_hover_index == i {
                Line::from(vec![Span::styled(format!("> {}", interface).to_string(), Style::default().add_modifier(Modifier::BOLD))])
            } else {
                Line::from(interface.to_string())
            };
            ListItem::new(content)
        }).collect();
    
        let interface_list = List::new(interface_items)
            .block(Block::default().borders(Borders::NONE));
    
        interfaces_block.render(inner_area, buf);
        frame.render_widget(interface_list, interface_area);
    }
    

    fn running_render_frame(&mut self, frame: &mut Frame) {
        let area = frame.size();
        let buf = frame.buffer_mut();

        let interface_name = self.chosen_interface.as_ref().unwrap();

        let title = Title::from(format!(" NETCHECK | {} ", interface_name).bold());
        let instructions = Title::from(Line::from(vec![" Quit ".into(), "<Q> ".blue().bold()]));
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

        let columns = inner_area.width / BLOCK_WIDTH;
        let column_width = inner_area.width / columns;
        self.block_width_practice = column_width as u32;

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
        let now = std::time::Instant::now();

        while now.elapsed() < std::time::Duration::from_millis(50) {
            match event::poll(std::time::Duration::from_millis(50))? {
                true => {
                    return match event::read()? {
                        // it's important to check that the event is a key press event as
                        // crossterm also emits key release and repeat events on Windows.
                        Event::Key(key_event) if key_event.kind == KeyEventKind::Press => self
                            .handle_key_event(key_event)
                            .wrap_err_with(|| format!("handling key event failed:\n{key_event:#?}")),
                        _ => Ok(()),
                    };
                }
                false => {
                    break;
                }
            }
        }

        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<()> {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Char('Q') => self.exit(),
            KeyCode::Up => {
                if let ApplicationStage::PickInterface = self.stage {
                    if self.interface_hover_index > 0 {
                        self.interface_hover_index -= 1;
                    }
                }
            },
            KeyCode::Down => {
                if let ApplicationStage::PickInterface = self.stage {
                    if self.interface_hover_index < self.interface_list.len() - 1 {
                        self.interface_hover_index += 1;
                    }
                }
            },
            KeyCode::Enter => {
                if let ApplicationStage::PickInterface = self.stage {
                    self.chosen_interface = Some(self.interface_list[self.interface_hover_index].clone());
                    self.stage = ApplicationStage::Running;

                    // Initialise fetching of network information
                    self.initialise_interface_fetching();
                }
            },
            _ => {}
        }
        Ok(())
    }

    fn initialise_interface_fetching(&mut self) {
        let (send, receive): (Sender<FetchedDataMessage>, Receiver<FetchedDataMessage>) = mpsc::channel();

        self.receive_new_data_channel = Some(receive);

        let chosen_interface = self.chosen_interface.clone().unwrap();

        let send_1 = send.clone();
        let chosen_interface_1 = chosen_interface.clone();

        thread::spawn(move || {
            fetch_local::fetch_and_return_local_info(send_1, chosen_interface_1);
        });

        thread::spawn(move || {
            fetch_dns::fetch_and_return_dns_info(send, chosen_interface);
        });
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn render_network_info(&self, area: Rect) -> Paragraph {
        let mut text = Vec::with_capacity(3);

        let max_width = self.block_width_practice as usize - 2;
        
        match &self.network_info.local_info.local_ip {
            Some(local_ip) => {
                let ip_str = local_ip.to_string();
                let padding = max_width.saturating_sub("Local IP: ".len() + ip_str.len());

                text.push(Line::from(vec![
                    Span::styled("Local IP: ", Style::default().bold()),
                    Span::raw(" ".repeat(padding)),
                    Span::styled(ip_str, Style::default().fg(Color::Green)),
                ]));
            }
            None => {
                let padding = max_width.saturating_sub("Local IP: Unknown".len());
                text.push(Line::from(vec![
                    Span::styled("Local IP: ", Style::default().bold()),
                    Span::raw(" ".repeat(padding)),
                    Span::styled("Unknown", Style::default().fg(Color::Red)),
                ]));
            }
        }
    
        match &self.network_info.local_info.subnet_mask {
            Some(subnet_mask) => {
                let mask_str = subnet_mask.to_string();
                let padding = max_width.saturating_sub("Subnet Mask: ".len() + mask_str.len());
                text.push(Line::from(vec![
                    Span::styled("Subnet Mask: ", Style::default().bold()),
                    Span::raw(" ".repeat(padding)),
                    Span::styled(mask_str, Style::default().fg(Color::Green)),
                ]));
            }
            None => {
                let padding = max_width.saturating_sub("Subnet Mask: Unknown".len());
                text.push(Line::from(vec![
                    Span::styled("Subnet Mask: ", Style::default().bold()),
                    Span::raw(" ".repeat(padding)),
                    Span::styled("Unknown", Style::default().fg(Color::Red)),
                ]));
            }
        }
    
        match &self.network_info.local_info.gateway {
            Some(gateway) => {
                let gateway_str = gateway.to_string();
                let padding = max_width.saturating_sub("Gateway: ".len() + gateway_str.len());
                text.push(Line::from(vec![
                    Span::styled("Gateway: ", Style::default().bold()),
                    Span::raw(" ".repeat(padding)),
                    Span::styled(gateway_str, Style::default().fg(Color::Green)),
                ]));
            }
            None => {
                let padding = max_width.saturating_sub("Gateway: Unknown".len());
                text.push(Line::from(vec![
                    Span::styled("Gateway: ", Style::default().bold()),
                    Span::raw(" ".repeat(padding)),
                    Span::styled("Unknown", Style::default().fg(Color::Red)),
                ]));
            }
        }

        let title = Span::styled("Network Info", Style::default().add_modifier(Modifier::BOLD));
    
        Paragraph::new(Text::from(text))
            .block(Block::default().title(title).borders(Borders::ALL))
    }

    fn render_internet_info(&self, _area: Rect) -> Paragraph {
        let text = vec![
            Line::from("Public IP: 203.0.113.1"),
            Line::from("ASN: 12345"),
            Line::from("Reverse DNS: example.com"),
            Line::from("ISP: Example ISP"),
            Line::from("Location: Somewhere, Earth"),
            Line::from("Cloudflare Ping: 1ms")
        ];
        Paragraph::new(Text::from(text)).block(
            Block::default()
                .title("Internet Info")
                .borders(Borders::ALL),
        )
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
        if self.network_info.dns_info.can_fetch == None {
            return Paragraph::new(Text::from(vec![Line::from("Fetching list...")]))
                .block(Block::default().title("DNS Info").borders(Borders::ALL));
        }

        if self.network_info.dns_info.can_fetch == Some(false) {
            return Paragraph::new(Text::from(vec![Line::from("Failed to get list.")]))
                .block(Block::default().title("DNS Info").borders(Borders::ALL));
        }

        let mut text = Vec::new();

        let max_width = self.block_width_practice as usize - 2;

        if self.network_info.dns_info.dns_servers.len() == 0 {
            text.push(Line::from("No DNS servers found."));
        } else {
            text.push(Line::from(vec![Span::styled("Servers:", Style::default().bold())]));

            for server in &self.network_info.dns_info.dns_servers {
                let colour = match server.can_resolve {
                    Some(true) => Color::Green,
                    Some(false) => Color::Red,
                    None => Color::Yellow,
                };

                let message = match server.can_resolve {
                    Some(true) => "OK",
                    Some(false) => "Failure",
                    None => "Waiting",
                };

                let padding = max_width.saturating_sub(server.ip.len() + message.len());

                text.push(Line::from(vec![
                    Span::styled(format!("{}{}{}", server.ip.to_string(), " ".repeat(padding), message), Style::default().fg(colour)),
                ]));
            }
        }

        Paragraph::new(Text::from(text))
            .block(Block::default().title("DNS Info").borders(Borders::ALL))
    }

    fn render_traceroute_info(&self, _area: Rect) -> Paragraph {
        let text = vec![
            Line::from("Hop 1: 192.168.0.1 - Latency: 1ms"),
            Line::from("Hop 2: 203.0.113.1 - Latency: 10ms"),
            Line::from("Hop 3: 198.51.100.1 - Latency: 20ms"),
        ];
        Paragraph::new(Text::from(text)).block(
            Block::default()
                .title("Traceroute Info")
                .borders(Borders::ALL),
        )
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
