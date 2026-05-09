use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Gauge, Paragraph, Tabs},
};
use std::sync::mpsc::{Receiver, channel};
use std::thread;
use std::{error::Error, io, time::Duration};

mod core;

fn get_config_path() -> std::path::PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    path.push("eidolon_config.txt");
    path
}

fn load_config() -> (String, u8, bool, bool, bool, bool, bool) {
    let (mut k, mut hw, mut mc, mut fec, mut sync, mut zstd, mut aud) =
        (String::from("NAVI"), 0, true, true, true, true, false);
    if let Ok(content) = std::fs::read_to_string(get_config_path()) {
        for line in content.lines() {
            let parts: Vec<&str> = line.split('=').collect();
            if parts.len() == 2 {
                let (key, val) = (parts[0].trim(), parts[1].trim());
                match key {
                    "key" => k = val.to_string(),
                    "hw" => hw = val.parse().unwrap_or(0),
                    "mc" => mc = val == "true",
                    "fec" => fec = val == "true",
                    "sync" => sync = val == "true",
                    "zstd" => zstd = val == "true",
                    "audio" => aud = val == "true",
                    _ => {}
                }
            }
        }
    }
    (k, hw, mc, fec, sync, zstd, aud)
}

fn save_config(k: &str, hw: u8, mc: bool, fec: bool, sync: bool, zstd: bool, aud: bool) {
    let content = format!(
        "key={}\nhw={}\nmc={}\nfec={}\nsync={}\nzstd={}\naudio={}\n",
        k, hw, mc, fec, sync, zstd, aud
    );
    let _ = std::fs::write(get_config_path(), content);
}

struct App {
    pub tabs: Vec<&'static str>,
    pub active_tab: usize,
    pub should_quit: bool,
    pub logs: Vec<String>,
    pub encryption_key: String,
    pub is_editing_key: bool,
    pub settings_cursor: usize,

    pub hw_accel: u8,
    pub use_multicore: bool,
    pub use_fec: bool,
    pub use_sync: bool,
    pub use_zstd: bool,
    pub use_audio: bool,

    pub is_processing: bool,
    pub progress: f32,
    pub job_rx: Option<Receiver<crate::core::JobMsg>>,
}

impl App {
    fn new() -> App {
        let (k, hw, mc, fec, sync, zstd, aud) = load_config();
        App {
            tabs: vec![
                "[1] ENCODE",
                "[2] DECODE",
                "[3] SETTINGS",
                "[4] SYSTEM LOGS",
            ],
            active_tab: 0,
            should_quit: false,
            logs: vec![
                "EIDOLON Kernel Initialized.".to_string(),
                "L.I.N.E. Protocol (Luma-Isolated Network Encoding) Active.".to_string(),
            ],
            encryption_key: k,
            is_editing_key: false,
            settings_cursor: 0,
            hw_accel: hw,
            use_multicore: mc,
            use_fec: fec,
            use_sync: sync,
            use_zstd: zstd,
            use_audio: aud,
            is_processing: false,
            progress: 0.0,
            job_rx: None,
        }
    }
    pub fn next_tab(&mut self) {
        self.active_tab = (self.active_tab + 1) % self.tabs.len();
    }
    pub fn prev_tab(&mut self) {
        if self.active_tab > 0 {
            self.active_tab -= 1;
        } else {
            self.active_tab = self.tabs.len() - 1;
        }
    }
    pub fn log(&mut self, message: &str) {
        self.logs.push(message.to_string());
    }
    pub fn save(&self) {
        save_config(
            &self.encryption_key,
            self.hw_accel,
            self.use_multicore,
            self.use_fec,
            self.use_sync,
            self.use_zstd,
            self.use_audio,
        );
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    if let Err(err) = res {
        println!("{:?}", err)
    }
    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()>
where
    std::io::Error: From<B::Error>,
{
    loop {
        terminal.draw(|f| ui(f, app))?;

        if let Some(rx) = app.job_rx.take() {
            let mut keep_rx = true;
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    crate::core::JobMsg::Progress(p) => app.progress = p,
                    crate::core::JobMsg::Log(l) => app.log(&l),
                    crate::core::JobMsg::Done(m) => {
                        app.log(&m);
                        app.is_processing = false;
                        keep_rx = false;
                        break;
                    }
                    crate::core::JobMsg::Error(e) => {
                        app.log(&format!("CRITICAL: {}", e));
                        app.is_processing = false;
                        keep_rx = false;
                        break;
                    }
                }
            }
            if keep_rx {
                app.job_rx = Some(rx);
            }
        }

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press && !app.is_processing {
                    if app.is_editing_key {
                        match key.code {
                            KeyCode::Char(c) => app.encryption_key.push(c),
                            KeyCode::Backspace => {
                                app.encryption_key.pop();
                            }
                            KeyCode::Enter | KeyCode::Esc => {
                                app.is_editing_key = false;
                                app.save();
                            }
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('q')
                            | KeyCode::Char('Q')
                            | KeyCode::Char('й')
                            | KeyCode::Char('Й')
                            | KeyCode::Esc => app.should_quit = true,
                            KeyCode::Right | KeyCode::Tab => app.next_tab(),
                            KeyCode::Left => app.prev_tab(),
                            KeyCode::Char('1') => app.active_tab = 0,
                            KeyCode::Char('2') => app.active_tab = 1,
                            KeyCode::Char('3') => app.active_tab = 2,
                            KeyCode::Char('4') => app.active_tab = 3,
                            KeyCode::Up => {
                                if app.active_tab == 2 {
                                    app.settings_cursor = app.settings_cursor.saturating_sub(1);
                                }
                            }
                            KeyCode::Down => {
                                if app.active_tab == 2 && app.settings_cursor < 6 {
                                    app.settings_cursor += 1;
                                }
                            }

                            KeyCode::Char('f')
                            | KeyCode::Char('F')
                            | KeyCode::Char('а')
                            | KeyCode::Char('А') => {
                                if app.active_tab == 0 {
                                    if let Some(input_path) = rfd::FileDialog::new().pick_folder() {
                                        let folder_name = input_path
                                            .file_name()
                                            .unwrap_or_default()
                                            .to_string_lossy()
                                            .to_string();
                                        let filename = format!("{}.tar", folder_name);

                                        if let Some(output_path) = rfd::FileDialog::new()
                                            .set_file_name(&format!("{}.mp4", folder_name))
                                            .add_filter("Video Stream", &["mp4"])
                                            .save_file()
                                        {
                                            let filepath = input_path.to_string_lossy().to_string();
                                            let out_path_str =
                                                output_path.to_string_lossy().to_string();
                                            let key = app.encryption_key.clone();
                                            let (fec, mc, sync, zstd, audio, hw) = (
                                                app.use_fec,
                                                app.use_multicore,
                                                app.use_sync,
                                                app.use_zstd,
                                                app.use_audio,
                                                app.hw_accel,
                                            );

                                            let (tx, rx) = channel();
                                            app.job_rx = Some(rx);
                                            app.is_processing = true;
                                            app.progress = 0.0;
                                            thread::spawn(
                                                move || match crate::core::encode_process(
                                                    filepath,
                                                    true,
                                                    out_path_str.clone(),
                                                    filename,
                                                    key,
                                                    fec,
                                                    mc,
                                                    sync,
                                                    zstd,
                                                    audio,
                                                    hw,
                                                    tx.clone(),
                                                ) {
                                                    Ok(_) => {
                                                        let _ = tx.send(crate::core::JobMsg::Done(
                                                            format!(
                                                                "SUCCESS: Stream assembled at '{}'",
                                                                out_path_str
                                                            ),
                                                        ));
                                                    }
                                                    Err(e) => {
                                                        let _ =
                                                            tx.send(crate::core::JobMsg::Error(e));
                                                    }
                                                },
                                            );
                                        }
                                    }
                                }
                            }

                            KeyCode::Char('u')
                            | KeyCode::Char('U')
                            | KeyCode::Char('г')
                            | KeyCode::Char('Г') => {
                                if app.active_tab == 1 {
                                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                        if let Ok(url) = clipboard.get_text() {
                                            if url.starts_with("http") {
                                                if let Some(output_dir) =
                                                    rfd::FileDialog::new().pick_folder()
                                                {
                                                    let out_dir_str =
                                                        output_dir.to_string_lossy().to_string();
                                                    let key = app.encryption_key.clone();
                                                    let (fec, mc, sync, hw) = (
                                                        app.use_fec,
                                                        app.use_multicore,
                                                        app.use_sync,
                                                        app.hw_accel,
                                                    );

                                                    let (tx, rx) = channel();
                                                    app.job_rx = Some(rx);
                                                    app.is_processing = true;
                                                    app.progress = 0.0;

                                                    thread::spawn(move || {
                                                        match crate::core::decode_url_process(
                                                            url,
                                                            out_dir_str.clone(),
                                                            key,
                                                            fec,
                                                            mc,
                                                            sync,
                                                            hw,
                                                            tx.clone(),
                                                        ) {
                                                            Ok(_) => {
                                                                let _ = tx.send(crate::core::JobMsg::Done(format!("SUCCESS: Target extracted to '{}'", out_dir_str)));
                                                            }
                                                            Err(e) => {
                                                                let _ = tx.send(
                                                                    crate::core::JobMsg::Error(e),
                                                                );
                                                            }
                                                        }
                                                    });
                                                }
                                            } else {
                                                app.log(
                                                    "ERROR: Invalid URL protocol in clipboard.",
                                                );
                                            }
                                        } else {
                                            app.log("ERROR: Clipboard memory void.");
                                        }
                                    } else {
                                        app.log("ERROR: Clipboard interface inaccessible.");
                                    }
                                }
                            }

                            KeyCode::Enter | KeyCode::Char(' ') => {
                                if app.active_tab == 2 {
                                    match app.settings_cursor {
                                        0 => app.is_editing_key = true,
                                        1 => {
                                            app.hw_accel = (app.hw_accel + 1) % 3;
                                            app.save();
                                        }
                                        2 => {
                                            app.use_multicore = !app.use_multicore;
                                            app.save();
                                        }
                                        3 => {
                                            app.use_fec = !app.use_fec;
                                            app.save();
                                        }
                                        4 => {
                                            app.use_sync = !app.use_sync;
                                            app.save();
                                        }
                                        5 => {
                                            app.use_zstd = !app.use_zstd;
                                            app.save();
                                        }
                                        6 => {
                                            app.use_audio = !app.use_audio;
                                            app.save();
                                        }
                                        _ => {}
                                    }
                                } else if app.active_tab == 0 {
                                    if let Some(input_path) = rfd::FileDialog::new().pick_file() {
                                        let filename = input_path
                                            .file_name()
                                            .unwrap_or_default()
                                            .to_string_lossy()
                                            .to_string();
                                        if let Some(output_path) = rfd::FileDialog::new()
                                            .set_file_name(&format!("{}.mp4", filename))
                                            .add_filter("Video Stream", &["mp4"])
                                            .save_file()
                                        {
                                            let filepath = input_path.to_string_lossy().to_string();
                                            let out_path_str =
                                                output_path.to_string_lossy().to_string();
                                            let key = app.encryption_key.clone();
                                            let (fec, mc, sync, zstd, audio, hw) = (
                                                app.use_fec,
                                                app.use_multicore,
                                                app.use_sync,
                                                app.use_zstd,
                                                app.use_audio,
                                                app.hw_accel,
                                            );

                                            let (tx, rx) = channel();
                                            app.job_rx = Some(rx);
                                            app.is_processing = true;
                                            app.progress = 0.0;
                                            thread::spawn(
                                                move || match crate::core::encode_process(
                                                    filepath,
                                                    false,
                                                    out_path_str.clone(),
                                                    filename,
                                                    key,
                                                    fec,
                                                    mc,
                                                    sync,
                                                    zstd,
                                                    audio,
                                                    hw,
                                                    tx.clone(),
                                                ) {
                                                    Ok(_) => {
                                                        let _ = tx.send(crate::core::JobMsg::Done(
                                                            format!(
                                                                "SUCCESS: Stream assembled at '{}'",
                                                                out_path_str
                                                            ),
                                                        ));
                                                    }
                                                    Err(e) => {
                                                        let _ =
                                                            tx.send(crate::core::JobMsg::Error(e));
                                                    }
                                                },
                                            );
                                        }
                                    }
                                } else if app.active_tab == 1 {
                                    if let Some(video_path) = rfd::FileDialog::new()
                                        .add_filter("Video Stream", &["mp4", "mkv", "webm"])
                                        .pick_file()
                                    {
                                        if let Some(output_dir) =
                                            rfd::FileDialog::new().pick_folder()
                                        {
                                            let video_str =
                                                video_path.to_string_lossy().to_string();
                                            let out_dir_str =
                                                output_dir.to_string_lossy().to_string();
                                            let key = app.encryption_key.clone();
                                            let (fec, mc, sync, hw) = (
                                                app.use_fec,
                                                app.use_multicore,
                                                app.use_sync,
                                                app.hw_accel,
                                            );

                                            let (tx, rx) = channel();
                                            app.job_rx = Some(rx);
                                            app.is_processing = true;
                                            app.progress = 0.0;
                                            thread::spawn(
                                                move || match crate::core::decode_process(
                                                    video_str,
                                                    out_dir_str.clone(),
                                                    key,
                                                    fec,
                                                    mc,
                                                    sync,
                                                    hw,
                                                    tx.clone(),
                                                ) {
                                                    Ok(_) => {
                                                        let _ = tx.send(crate::core::JobMsg::Done(
                                                            format!(
                                                                "SUCCESS: Target extracted to '{}'",
                                                                out_dir_str
                                                            ),
                                                        ));
                                                    }
                                                    Err(e) => {
                                                        let _ =
                                                            tx.send(crate::core::JobMsg::Error(e));
                                                    }
                                                },
                                            );
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        if app.should_quit {
            return Ok(());
        }
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}

fn ui(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();
    let main_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .title("[ EIDOLON // BY TUFTA ] ")
        .border_style(Style::default().fg(Color::DarkGray))
        .title_style(Style::default().fg(Color::Gray));
    f.render_widget(main_block, area);

    let inner_area = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(6),
            ]
            .as_ref(),
        )
        .split(area);

    let titles = app
        .tabs
        .iter()
        .map(|t| Line::from(Span::styled(*t, Style::default().fg(Color::DarkGray))));
    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .highlight_style(
            Style::default()
                .fg(Color::White)
                .add_modifier(ratatui::style::Modifier::BOLD),
        )
        .select(app.active_tab);
    f.render_widget(tabs, inner_area[0]);

    let content = match app.active_tab {
        0 => render_encode_tab(),
        1 => render_decode_tab(),
        2 => render_settings_tab(app),
        _ => render_logs_tab(app),
    };
    f.render_widget(content, inner_area[1]);

    let status_text: Vec<Line> = app
        .logs
        .iter()
        .skip(app.logs.len().saturating_sub(4))
        .map(|log| {
            Line::from(vec![
                Span::styled(">[SYS]", Style::default().fg(Color::DarkGray)),
                Span::raw(format!(" {}", log)),
            ])
        })
        .collect();
    let status_block = Paragraph::new(status_text)
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(" TERMINAL STREAM "),
        )
        .style(Style::default().fg(Color::Gray));
    f.render_widget(status_block, inner_area[2]);

    if app.is_processing {
        let popup_area = centered_rect(60, 20, area);
        f.render_widget(Clear, popup_area);
        let gauge = Gauge::default()
            .block(
                Block::default()
                    .title(" L.I.N.E. SUBSTRATUM UPLINK ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::White)),
            )
            .gauge_style(Style::default().fg(Color::White).bg(Color::DarkGray))
            .ratio((app.progress.clamp(0.0, 1.0)) as f64);
        f.render_widget(gauge, popup_area);
    }
}

fn render_encode_tab<'a>() -> Paragraph<'a> {
    let t = vec![
        Line::from("TARGET INJECTION"),
        Line::from("------------------"),
        Line::from(""),
        Line::from(Span::styled(
            "Press [Space] to select FILE payload.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "Press[F] to select DIRECTORY payload.",
            Style::default().fg(Color::DarkGray),
        )),
    ];
    Paragraph::new(t)
        .style(Style::default().fg(Color::Gray))
        .block(Block::default().padding(ratatui::widgets::Padding::new(2, 2, 1, 1)))
}
fn render_decode_tab<'a>() -> Paragraph<'a> {
    let t = vec![
        Line::from("SUBSTRATUM EXTRACTION"),
        Line::from("---------------------"),
        Line::from(""),
        Line::from(Span::styled(
            "Press [Space] to extract from LOCAL STREAM.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "Press [U] to intercept from URL in CLIPBOARD.",
            Style::default().fg(Color::DarkGray),
        )),
    ];
    Paragraph::new(t)
        .style(Style::default().fg(Color::Gray))
        .block(Block::default().padding(ratatui::widgets::Padding::new(2, 2, 1, 1)))
}
fn render_settings_tab<'a>(app: &'a App) -> Paragraph<'a> {
    let cursor = |idx| {
        if app.settings_cursor == idx {
            "> "
        } else {
            "  "
        }
    };
    let key_text = format!(
        "{}ACCESS KEY: {}{}",
        cursor(0),
        app.encryption_key,
        if app.is_editing_key { "█" } else { "" }
    );
    let key_style = if app.is_editing_key {
        Style::default()
            .fg(Color::White)
            .add_modifier(ratatui::style::Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };
    let hw_text = match app.hw_accel {
        1 => "NVENC (Nvidia API)",
        2 => "AMF (AMD API)",
        _ => "CPU (libx264)",
    };

    let description = match app.settings_cursor {
        0 => "AES-256-GCM symmetric key. Protects the payload from external parsing.",
        1 => "Hardware Acceleration. Bypasses CPU bottleneck during video muxing.",
        2 => "Rayon Framework. Allocates all available CPU threads for pixel matrix generation.",
        3 => {
            "Reed-Solomon FEC. Generates parity matrices to counter heavy CDN compression artifacts."
        }
        4 => "Deterministic ID Injection. Prevents matrix collapse caused by CDN frame drops.",
        5 => "Zstandard Algorithm. Compresses entropy prior to encryption, maximizing CDN density.",
        6 => {
            "FSK Synthesis. Generates a data-driven acoustic layer to bypass automated CDN spam filters."
        }
        _ => "",
    };

    let t = vec![
        Line::from("PROTOCOL PARAMETERS"),
        Line::from("-------------------"),
        Line::from(Span::styled(key_text, key_style)),
        Line::from(""),
        Line::from(format!(
            "{}[{}] Engine Core: {}",
            cursor(1),
            if app.hw_accel > 0 { "X" } else { " " },
            hw_text
        )),
        Line::from(format!(
            "{}[{}] Matrix Threading (Multi-Core Render)",
            cursor(2),
            if app.use_multicore { "X" } else { " " }
        )),
        Line::from(format!(
            "{}[{}] Parity Injection (Reed-Solomon FEC)",
            cursor(3),
            if app.use_fec { "X" } else { " " }
        )),
        Line::from(format!(
            "{}[{}] Frame Synchronization (Anti-Drop Hook)",
            cursor(4),
            if app.use_sync { "X" } else { " " }
        )),
        Line::from(format!(
            "{}[{}] Entropy Compression (Zstandard Pre-Pass)",
            cursor(5),
            if app.use_zstd { "X" } else { " " }
        )),
        Line::from(format!(
            "{}[{}] Acoustic Modem Track (Anti-Spam Bypass)",
            cursor(6),
            if app.use_audio { "X" } else { " " }
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("DATA: {}", description),
            Style::default().fg(Color::DarkGray),
        )),
    ];
    Paragraph::new(t)
        .style(Style::default().fg(Color::Gray))
        .block(Block::default().padding(ratatui::widgets::Padding::new(2, 2, 1, 1)))
}
fn render_logs_tab<'a>(app: &'a App) -> Paragraph<'a> {
    let mut t = vec![
        Line::from("TERMINAL DIAGNOSTICS"),
        Line::from("--------------------"),
    ];
    for log in app.logs.iter().skip(app.logs.len().saturating_sub(20)) {
        t.push(Line::from(format!("> {}", log)));
    }
    Paragraph::new(t)
        .style(Style::default().fg(Color::Gray))
        .block(Block::default().padding(ratatui::widgets::Padding::new(2, 2, 1, 1)))
}
