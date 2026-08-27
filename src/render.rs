// src/rendering.rs
//
// Rendering.rs is responsible for rendering the Olympus interface and its pages.

use super::app::{App, Screen};
use super::math::{coggan_pwr_model, olt_hr_model, zone2color};
use super::nav::{MainSelection, SettingsSelection};

use chrono::Local;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::symbols;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Gauge, Paragraph, Sparkline};
use tui_big_text::{BigText, PixelSize};

// ====================================
// --------- Helper Functions ---------
// ====================================

// Footer rendering function
fn footer(current: Screen, app: &App) -> Paragraph<'_> {
    let hlgt_stl = Style::default().fg(Color::Green); // Highlighted style
    let bhlgt_stl = Style::default().add_modifier(Modifier::BOLD); // Bold highlighted style
    let sep = Span::styled(" | ", hlgt_stl.fg(Color::Gray));

    // Initialize un-highlighted
    let mainspan = Span::styled(" Main page", bhlgt_stl);
    let conspan = Span::styled(" Control panel", bhlgt_stl);
    let dbspan = Span::styled(" Database", bhlgt_stl);
    let setspan = Span::styled(" Settings", bhlgt_stl);
    let statsspan = Span::styled(" Stats", bhlgt_stl);

    let userspan = Span::styled(app.user(), hlgt_stl);
    //let connspan = Span::styled(app.connection(), hlgt_stl);

    let current_page: Span<'_>;

    // Highlight current screen
    match current {
        Screen::Main => current_page = mainspan,
        Screen::Control => current_page = conspan,
        Screen::Database => current_page = dbspan,
        Screen::Settings => current_page = setspan,
        Screen::Stats => current_page = statsspan,
    }

    let footerspan = vec![current_page, sep.clone(), userspan, sep.clone()];
    let footerline = Line::from(footerspan);
    let footerblock = Block::default()
        .borders(Borders::LEFT | Borders::RIGHT | Borders::TOP)
        .fg(Color::DarkGray)
        .border_type(BorderType::Rounded);

    Paragraph::new(footerline).block(footerblock)
}

fn header(app: &App) -> Paragraph<'_> {
    let headerline = Line::from(vec![
        Span::from(" "),
        Span::from("olympus").fg(Color::Cyan),
        Span::from(" "),
        Span::from(format!("{}", app.version())).fg(Color::White),
        Span::from(" "),
        Span::from(format!("{}", Local::now())).fg(Color::White),
    ]);
    let headerblock = Block::default()
        .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
        .fg(Color::DarkGray)
        .border_type(BorderType::Rounded);

    Paragraph::new(headerline).block(headerblock)
}

// ====================================
// --- Page-Specific Draw Functions ---
// ====================================

fn main_draw(frame: &mut Frame, area: Rect, app: &App) {
    let _livedata = app.livedata();
    let _userdata = app.userdata();

    let selected = app.selections().main();

    let gray = Style::default().fg(Color::Gray);
    let cyan = Style::default().fg(Color::Cyan);
    let red = Style::default().fg(Color::Red);

    // --- MAIN MENU PANEL ---
    // Split the main space vertically to center-align the menu components
    let [_, ascii, _, navopt] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Spacer
            Constraint::Length(6), // OLYMPUS ASCII Logo
            Constraint::Length(1), // Spacer
            Constraint::Min(3),    // Navigation Options
        ])
        .areas(area);

    // Multi-line slant ASCII title
    let ascii_logo = vec![
        Line::from(Span::styled(
            "  ____  _ __     ____  __ ____  _   _ ____  ",
            cyan,
        )),
        Line::from(Span::styled(
            " / __ \\| |\\ \\   / /  \\/  |  _ \\| | | / ___| ",
            cyan,
        )),
        Line::from(Span::styled(
            "| |  | | | \\ \\ / /| |\\/| | |_) | | | \\___ \\ ",
            cyan,
        )),
        Line::from(Span::styled(
            "| |__| | |__| \\ / | |  | |  __/| |_| |___) |",
            cyan,
        )),
        Line::from(Span::styled(
            " \\____/|_____|_|  |_|  |_|_|    \\___/|____/ ",
            cyan,
        )),
        Line::from(Span::styled(
            "   -- FREE OPEN-SOURCE TURBO TRAINER - BY DRACONIS --   ",
            gray,
        )),
    ];

    frame.render_widget(
        Paragraph::new(ascii_logo).alignment(Alignment::Center),
        ascii,
    );

    // Main menu selection text
    let mut menu_text = vec![
        Line::from(Span::styled("START NEW RIDE", gray)),
        Line::from(Span::styled("CONTROL PANEL", gray)),
        Line::from(Span::styled("WORKOUT DATABASE", gray)),
        Line::from(Span::styled("SETTINGS & SENSORS", gray)),
        Line::from(Span::styled("STATS & RECORDS", gray)),
        Line::from(Span::styled("QUIT", red)),
    ];

    // Highlight the selected menu item
    let selected_style = Style::default().fg(Color::White).bg(Color::Green);
    match selected {
        MainSelection::NewRide => menu_text[0].spans[0].style = selected_style,
        MainSelection::Control => menu_text[1].spans[0].style = selected_style,
        MainSelection::Workouts => menu_text[2].spans[0].style = selected_style,
        MainSelection::Settings => menu_text[3].spans[0].style = selected_style,
        MainSelection::Stats => menu_text[4].spans[0].style = selected_style,
        MainSelection::Quit => menu_text[5].spans[0].style = selected_style,
    }

    frame.render_widget(
        Paragraph::new(menu_text).alignment(Alignment::Center),
        navopt,
    );
}

// -------------------------------------------------------

fn control_draw(frame: &mut Frame, area: Rect, app: &App) {
    let livedata = app.livedata();
    let userdata = app.userdata();
    let userstats = &userdata.stats;

    // ============================================================
    // MAIN LAYOUT
    // ============================================================

    let [metrics, stats] =
        Layout::vertical([Constraint::Percentage(68), Constraint::Percentage(32)]).areas(area);

    // Big power panel + HR/cadence/speed
    let [pwrrect, right_metrics] =
        Layout::horizontal([Constraint::Percentage(42), Constraint::Percentage(58)]).areas(metrics);

    let [hrrect, bottom_metrics] =
        Layout::vertical([Constraint::Percentage(55), Constraint::Percentage(45)])
            .areas(right_metrics);

    let [rpmrect, velrect] =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .areas(bottom_metrics);

    // Bottom information row
    let [pwrzrect, statrect, intvlrect, sysrect] = Layout::horizontal([
        Constraint::Percentage(40),
        Constraint::Percentage(20),
        Constraint::Percentage(20),
        Constraint::Percentage(20),
    ])
    .areas(stats);

    // ============================================================
    // POWER
    // ============================================================

    let pwr_zone = coggan_pwr_model(livedata.crnt_pwr, userstats.ftp);
    let pwr_color = zone2color(pwr_zone);

    let pwr_percent = if userstats.ftp > 0 {
        (livedata.crnt_pwr as f32 / userstats.ftp as f32 * 100.0) as u16
    } else {
        0
    };

    let pwrblock = Block::default()
        .title(" POWER ")
        .borders(Borders::ALL)
        .fg(pwr_color)
        .border_type(BorderType::Rounded);

    let pwrinner = pwrblock.inner(pwrrect);
    frame.render_widget(pwrblock, pwrrect);

    let [pwr_header, pwr_main, pwr_graph, pwr_footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(6),
        Constraint::Fill(1),
        Constraint::Length(2),
    ])
    .areas(pwrinner);

    // --- Power header ---

    let pwr_header_line = Line::from(vec![
        Span::raw("ZONE "),
        Span::styled(
            format!("{}", pwr_zone),
            Style::default().fg(pwr_color).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  /  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("FTP {}W", userstats.ftp),
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    frame.render_widget(Paragraph::new(pwr_header_line), pwr_header);

    // --- Big power number ---

    let pwr_big = BigText::builder()
        .pixel_size(PixelSize::Full)
        .style(Style::default().fg(pwr_color))
        .lines(vec![format!("{}", livedata.crnt_pwr).into()])
        .build();

    frame.render_widget(pwr_big, pwr_main);

    // --- Power graph (rolling Sparkline) ---

    let pwr_history = app.power_history();
    let pwr_max = userstats.ftp.max(1) as u64;

    let pwr_spark = Sparkline::default()
        .block(Block::default().borders(Borders::NONE))
        .data(pwr_history.into_iter())
        .max(pwr_max)
        .bar_set(symbols::bar::NINE_LEVELS)
        .style(Style::default().fg(pwr_color));

    frame.render_widget(pwr_spark, pwr_graph);

    // --- Power footer ---

    let pwr_footer_lines = vec![
        Line::from(vec![
            Span::styled("TARGET ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}W", livedata.target_pwr),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("    "),
            Span::styled("AVG ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}W", livedata.avg_pwr),
                Style::default().fg(Color::White),
            ),
            Span::raw("    "),
            Span::styled("MAX ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}W", livedata.max_pwr),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("20m ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{}W", livedata.avg_20min_pwr)),
            Span::raw("    "),
            Span::styled("10m ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{}W", livedata.avg_10min_pwr)),
            Span::raw("    "),
            Span::styled("5m ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{}W", livedata.avg_5min_pwr)),
        ]),
    ];

    frame.render_widget(Paragraph::new(pwr_footer_lines), pwr_footer);

    // ============================================================
    // HEART RATE
    // ============================================================

    let hr_zone = olt_hr_model(livedata.crnt_hr, userstats.maxhr);

    let hr_percent = if userstats.maxhr > 0 {
        (livedata.crnt_hr as f32 / userstats.maxhr as f32 * 100.0) as u16
    } else {
        0
    };

    let hrblock = Block::default()
        .title(" HEART RATE ")
        .borders(Borders::ALL)
        .fg(Color::Red)
        .border_type(BorderType::Rounded);

    let hrinner = hrblock.inner(hrrect);
    frame.render_widget(hrblock, hrrect);

    let [hr_top, hr_graph, hr_bottom] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(hrinner);

    // --- Big heart rate number ---

    let hr_big = BigText::builder()
        .pixel_size(PixelSize::Full)
        .style(Style::default().fg(Color::Red))
        .lines(vec![format!("{} BPM", livedata.crnt_hr).into()])
        .build();

    frame.render_widget(hr_big, hr_top);

    // --- Heart rate graph ---

    let hr_history = app.hr_history();
    let hr_max = userstats.maxhr.max(1) as u64;

    let hr_spark = Sparkline::default()
        .block(Block::default().borders(Borders::NONE))
        .data(hr_history.into_iter())
        .max(hr_max)
        .bar_set(symbols::bar::NINE_LEVELS)
        .style(Style::default().fg(Color::Red));

    frame.render_widget(hr_spark, hr_graph);

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                "Z",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("{} ", hr_zone)),
            Span::styled("MAX", Style::default().fg(Color::DarkGray)),
            Span::raw(format!(" {}%", hr_percent)),
            Span::raw("    "),
            Span::styled("AVG ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{}", livedata.avg_hr)),
        ]))
        .alignment(Alignment::Center),
        hr_bottom,
    );

    // ============================================================
    // CADENCE
    // ============================================================

    let rpmblock = Block::default()
        .title(" CADENCE ")
        .borders(Borders::ALL)
        .fg(Color::LightBlue)
        .border_type(BorderType::Rounded);

    let rpminner = rpmblock.inner(rpmrect);
    frame.render_widget(rpmblock, rpmrect);

    let [rpm_value, rpm_gauge] =
        Layout::vertical([Constraint::Length(2), Constraint::Fill(1)]).areas(rpminner);

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!("{}", livedata.crnt_rpm),
                Style::default()
                    .fg(Color::LightBlue)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" RPM", Style::default().fg(Color::DarkGray)),
        ]))
        .alignment(Alignment::Center),
        rpm_value,
    );

    let rpm_ratio = (livedata.crnt_rpm as f64 / 120.0).clamp(0.0, 1.0);

    frame.render_widget(
        Gauge::default()
            .ratio(rpm_ratio)
            .label(format!(
                "AVG {}  MAX {}",
                livedata.avg_rpm, livedata.max_rpm
            ))
            .gauge_style(Style::default().fg(Color::LightBlue)),
        rpm_gauge,
    );

    // ============================================================
    // SPEED
    // ============================================================

    let velblock = Block::default()
        .title(" SPEED ")
        .borders(Borders::ALL)
        .fg(Color::Green)
        .border_type(BorderType::Rounded);

    let velinner = velblock.inner(velrect);
    frame.render_widget(velblock, velrect);

    let [vel_value, vel_gauge] =
        Layout::vertical([Constraint::Length(2), Constraint::Fill(1)]).areas(velinner);

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!("{:.1}", livedata.crnt_vel),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" KM/H", Style::default().fg(Color::DarkGray)),
        ]))
        .alignment(Alignment::Center),
        vel_value,
    );

    let speed_ratio = (livedata.crnt_vel as f64 / 60.0).clamp(0.0, 1.0);

    frame.render_widget(
        Gauge::default()
            .ratio(speed_ratio)
            .label(format!(
                "AVG {:.1}  MAX {:.1}",
                livedata.avg_vel, livedata.max_vel
            ))
            .gauge_style(Style::default().fg(Color::Green)),
        vel_gauge,
    );

    // ============================================================
    // POWER ZONES
    // ============================================================

    let pwrzblock = Block::default()
        .title(" POWER ZONES ")
        .borders(Borders::ALL)
        .fg(Color::DarkGray)
        .border_type(BorderType::Rounded);

    let pwrzinner = pwrzblock.inner(pwrzrect);
    frame.render_widget(pwrzblock, pwrzrect);

    let zone_layout: [Rect; 7] = Layout::vertical(vec![Constraint::Length(1); 7]).areas(pwrzinner);

    // Coggan boundaries as percentage of FTP.
    let zone_limits = [0.55, 0.75, 0.90, 1.05, 1.20, 1.50, 2.00];

    let current_ratio = if userstats.ftp > 0 {
        livedata.crnt_pwr as f64 / userstats.ftp as f64
    } else {
        0.0
    };

    for (i, rect) in zone_layout.iter().enumerate() {
        let lower = if i == 0 { 0.0 } else { zone_limits[i - 1] };

        let upper = zone_limits[i];

        let ratio = if current_ratio < lower {
            0.0
        } else if current_ratio >= upper {
            1.0
        } else {
            (current_ratio - lower) / (upper - lower)
        };

        let is_current = current_ratio >= lower && current_ratio < upper;

        let style = if is_current {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let label = if i == 0 {
            format!("Z1  <55%")
        } else {
            format!(
                "Z{}  {}-{}%",
                i + 1,
                (lower * 100.0) as u16,
                (upper * 100.0) as u16,
            )
        };

        frame.render_widget(
            Gauge::default()
                .ratio(ratio.clamp(0.0, 1.0))
                .label(label)
                .gauge_style(style),
            *rect,
        );
    }

    // ============================================================
    // RIDE STATS
    // ============================================================

    let statblock = Block::default()
        .title(" RIDE STATS ")
        .borders(Borders::ALL)
        .fg(Color::DarkGray)
        .border_type(BorderType::Rounded);

    let statinner = statblock.inner(statrect);
    frame.render_widget(statblock, statrect);

    let hours = livedata.elapsed_secs / 3600;
    let minutes = (livedata.elapsed_secs / 60) % 60;
    let seconds = livedata.elapsed_secs % 60;

    let stattext = Paragraph::new(vec![
        Line::from(format!(
            "TIME       {:02}:{:02}:{:02}",
            hours, minutes, seconds
        )),
        Line::from(format!("DIST       {:.1} km", livedata.elapsed_distance)),
        Line::from(format!("ELEV       {} m", livedata.alti)),
        Line::from(format!("GRAD       {:.1}%", livedata.grad)),
        Line::from(format!("CAL        {} kcal", livedata.calories)),
        Line::from(format!("TSS        {}", livedata.tss)),
        Line::from(format!("IF         {:.2}", livedata.ifac)),
    ]);

    frame.render_widget(stattext, statinner);

    // ============================================================
    // INTERVALS
    // ============================================================

    let intvlblock = Block::default()
        .title(" INTERVALS ")
        .borders(Borders::ALL)
        .fg(Color::DarkGray)
        .border_type(BorderType::Rounded);

    let intvlinner = intvlblock.inner(intvlrect);
    frame.render_widget(intvlblock, intvlrect);

    frame.render_widget(
        Paragraph::new(vec![
            Line::from("CURRENT"),
            Line::from(""),
            Line::from(vec![
                Span::styled("TARGET ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{}W", livedata.target_pwr),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from("Workout data"),
        ])
        .alignment(Alignment::Center),
        intvlinner,
    );

    // ============================================================
    // SYSTEM
    // ============================================================

    let sysblock = Block::default()
        .title(" SYSTEM ")
        .borders(Borders::ALL)
        .fg(Color::DarkGray)
        .border_type(BorderType::Rounded);

    let sysinner = sysblock.inner(sysrect);
    frame.render_widget(sysblock, sysrect);

    // We only use values that are currently exposed by the
    // function's App API. Trainer/device connection state can
    // be dropped in here later when those fields are exposed.

    let systext = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("FTP       ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{} W", userstats.ftp)),
        ]),
        Line::from(vec![
            Span::styled("MAX HR    ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{} BPM", userstats.maxhr)),
        ]),
        Line::from(vec![
            Span::styled("POWER     ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{} W", livedata.crnt_pwr),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::styled("HR        ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{} BPM", livedata.crnt_hr),
                Style::default().fg(Color::Red),
            ),
        ]),
        Line::from(vec![
            Span::styled("CADENCE   ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{} RPM", livedata.crnt_rpm)),
        ]),
    ]);

    frame.render_widget(systext, sysinner);
}

// -------------------------------------------------------

fn database_draw(frame: &mut Frame, area: Rect, app: &App) {
    let _selected = app.selections().database();

    // Colors
    let gray = Style::default().fg(Color::Gray);
    let dark_gray = Style::default().fg(Color::DarkGray);
    let white = Style::default().fg(Color::White);
    let bold_white = white.add_modifier(Modifier::BOLD);

    // Base Layout Structure
    let [sidebar_area, list_area] =
        Layout::horizontal([Constraint::Percentage(32), Constraint::Percentage(68)]).areas(area);

    let [searchbar_area, filters_area, preview_area, buttons_area] = Layout::vertical([
        Constraint::Length(3), // Search input block
        Constraint::Length(7), // Filter categories
        Constraint::Min(5),    // Formerly 'etc' - now a Workout Preview block!
        Constraint::Length(3), // Interactive hotkeys
    ])
    .areas(sidebar_area);

    // --- SIDEBAR BLOCK 1: SEARCH BAR ---
    let search_text = Line::from(vec![
        Span::styled(" Watopia", white),
        Span::styled("█", Style::default().fg(Color::Yellow)), // Simulated cursor
    ]);
    frame.render_widget(
        Paragraph::new(search_text).block(
            Block::new()
                .borders(Borders::ALL)
                .title(" 🔍 Search Workouts ")
                .fg(Color::DarkGray),
        ),
        searchbar_area,
    );

    // --- SIDEBAR BLOCK 2: FILTERS ---
    let filter_lines = vec![
        Line::from(vec![
            Span::styled(" 🔘 TYPE: ", gray),
            Span::styled(
                "[ Intervals ]",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  Tempo", gray),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" ⏳ DURATION: ", gray),
            Span::styled(" <30m", gray),
            Span::styled(
                "  [ 30-60m ]",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  60m+", gray),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(filter_lines).block(
            Block::new()
                .borders(Borders::ALL)
                .title(" 🎛️ Active Filters ")
                .fg(Color::DarkGray),
        ),
        filters_area,
    );

    // --- SIDEBAR BLOCK 3: SELECTED WORKOUT PREVIEW (Formerly etc) ---
    // Shows the exact power step breakdown for whatever row is currently picked
    let preview_lines = vec![
        Line::from(Span::styled(" \"SST Short\" Profile Preview:", gray)),
        Line::from(""),
        Line::from(vec![
            Span::styled(" ■ Warmup: ", Style::default().fg(Color::LightGreen)),
            Span::styled("10 mins ramping 100W -> 180W", white),
        ]),
        Line::from(vec![
            Span::styled(" ■ Work:   ", Style::default().fg(Color::Yellow)),
            Span::styled("3x 5 mins @ 240W (Zone 4)", white),
        ]),
        Line::from(vec![
            Span::styled(" ■ Rest:   ", Style::default().fg(Color::LightBlue)),
            Span::styled("3x 3 mins @ 140W (Zone 2)", white),
        ]),
        Line::from(vec![
            Span::styled(" ■ Cooldown:", Style::default().fg(Color::Green)),
            Span::styled(" 5 mins gradual recovery", white),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(preview_lines).block(
            Block::new()
                .borders(Borders::ALL)
                .title(" 📊 Profile Info ")
                .fg(Color::White),
        ),
        preview_area,
    );

    // --- SIDEBAR BLOCK 4: SIDEBAR BUTTONS ---
    let footer_buttons = Line::from(vec![
        Span::styled(" [Tab]", Style::default().fg(Color::Yellow)),
        Span::styled(" Switch Pane ", gray),
        Span::styled(" [R]", Style::default().fg(Color::Red)),
        Span::styled(" Reset Filters", gray),
    ]);
    frame.render_widget(
        Paragraph::new(footer_buttons)
            .alignment(Alignment::Center)
            .block(Block::new().borders(Borders::ALL).fg(Color::DarkGray)),
        buttons_area,
    );

    // --- MAIN BLOCK: WORKOUT LIST MATRIX ---
    // High-visibility table rows resembling a clean gaming catalog selection screen
    let workout_list = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                " ▶  [VO2 Max] ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::styled("Gorilla Intervals        ", bold_white),
            Span::styled("⏱ 45 mins  ", gray),
            Span::styled("⚡ 320 TSS", dark_gray),
        ]),
        Line::from(Span::styled(
            "     ↳ Focus: Burst power capacity & rapid clearance.",
            dark_gray,
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(" ⚡  [SweetSpot] ", Style::default().fg(Color::Yellow)),
            Span::styled("SST Short (Active Plan) ", white),
            Span::styled("⏱ 50 mins  ", gray),
            Span::styled("⚡ 210 TSS", dark_gray),
        ]),
        Line::from(Span::styled(
            "     ↳ Focus: Aerobic engine rebuilding without heavy exhaustion.",
            dark_gray,
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(" ⏱  [Recovery]  ", Style::default().fg(Color::LightBlue)),
            Span::styled("Active Flush             ", gray),
            Span::styled("⏱ 30 mins  ", gray),
            Span::styled("⚡ 080 TSS", dark_gray),
        ]),
    ];

    frame.render_widget(
        Paragraph::new(workout_list).block(
            Block::new()
                .borders(Borders::ALL)
                .title(" 🚴 Available Workout Modules ")
                .fg(Color::White),
        ),
        list_area,
    );
}

// -------------------------------------------------------

fn settings_draw(frame: &mut Frame, area: Rect, app: &App) {
    let selected = app.selections().settings();

    // Divide screen into primary structural blocks
    let [sidebar_area, controls_area] =
        Layout::horizontal([Constraint::Percentage(25), Constraint::Percentage(75)]).areas(area);

    // Structural Frame 1: Left Navigation Menu Container
    let nav_block = Block::default()
        .title(" Settings ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    // Structural Frame 2: Right Active Content Container
    let controls_block = Block::default()
        .title(" Configuration ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    // Render the outer structural borders
    frame.render_widget(nav_block, sidebar_area);
    frame.render_widget(controls_block, controls_area);

    // Inset the left margin slightly to separate border lines from inner text
    let inner_sidebar = sidebar_area.inner(Margin {
        horizontal: 1,
        vertical: 1,
    });

    // Slice the inner sidebar area vertically for text item groups
    let [general, appearance, bluetooth, system, user, _etc] = Layout::vertical([
        Constraint::Length(3), // Extra vertical height gives visual breathing room
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Min(0),
    ])
    .areas(inner_sidebar);

    // 5. Dynamic Style Helper: Creates an indicator block layout entirely via text styling
    let get_item_style = |selection: SettingsSelection| {
        if *selected == selection {
            // Selected item: Reverse video look or bold vivid color
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            // Inactive items: Dimmed text sitting cleanly against background
            Style::default().fg(Color::Gray).add_modifier(Modifier::DIM)
        }
    };

    // Construct borderless list menu options utilizing modern text prefix icons
    let generaltxt = Paragraph::new(Line::from(vec![Span::styled(
        if *selected == SettingsSelection::General {
            " -> General <- "
        } else {
            "    General    "
        },
        get_item_style(SettingsSelection::General),
    )]));

    let appeartxt = Paragraph::new(Line::from(vec![Span::styled(
        if *selected == SettingsSelection::Appearance {
            " -> Appearance <- "
        } else {
            "    Appearance    "
        },
        get_item_style(SettingsSelection::Appearance),
    )]));

    let bttxt = Paragraph::new(Line::from(vec![Span::styled(
        if *selected == SettingsSelection::Bluetooth {
            " -> Bluetooth <- "
        } else {
            "    Bluetooth    "
        },
        get_item_style(SettingsSelection::Bluetooth),
    )]));

    let systemtxt = Paragraph::new(Line::from(vec![Span::styled(
        if *selected == SettingsSelection::System {
            " -> System <- "
        } else {
            "    System    "
        },
        get_item_style(SettingsSelection::System),
    )]));

    let usertxt = Paragraph::new(Line::from(vec![Span::styled(
        if *selected == SettingsSelection::User {
            " -> User <- "
        } else {
            "    User    "
        },
        get_item_style(SettingsSelection::User),
    )]));

    // Render menu items directly onto the canvas frame
    frame.render_widget(generaltxt, general);
    frame.render_widget(appeartxt, appearance);
    frame.render_widget(bttxt, bluetooth);
    frame.render_widget(systemtxt, system);
    frame.render_widget(usertxt, user);

    // Draw actual content inside the Right Control Panel based on active selection
    let inner_controls_area = controls_area.inner(Margin {
        horizontal: 2,
        vertical: 1,
    });

    let content = match selected {
        SettingsSelection::General => Paragraph::new(format!(
            "General Settings\n----------------\n[ ] Auto-Save Enabled\n[ ] Check for Updates"
        )),
        SettingsSelection::Appearance => Paragraph::new(format!(
            "Appearance Settings\n-------------------\nTheme: Dark Mode\nFont Size: 12"
        )),
        SettingsSelection::Bluetooth => Paragraph::new(vec![
            Line::from("Bluetooth Devices"),
            Line::from("-----------------"),
            Line::from(app.connection()),
        ]),
        SettingsSelection::System => Paragraph::new(format!(
            "System Information\n------------------\nVersion: {}",
            app.version()
        )),
        SettingsSelection::User => Paragraph::new(vec![
            Line::from("User Settings"),
            Line::from("--------------"),
            Line::from(format!("[{}] Dark Mode", app.preferences().dark_mode)),
            Line::from(format!(
                "[{}] High Contrast",
                app.preferences().high_contrast
            )),
        ]),
    };

    frame.render_widget(content, inner_controls_area);
}

// -------------------------------------------------------

fn stats_draw(frame: &mut Frame, main_area: Rect, app: &App) {
    let _selected = app.selections().stats();
    let content = Paragraph::new(format!("Stats\n------\n"));
    frame.render_widget(content, main_area);
}

// ====================================
// --- Drawing Multiplexer Function ---
// ====================================

pub fn draw(frame: &mut Frame, app: &App) {
    // Properly split the screen area into two horizontal sections
    let [header_area, main_area, footer_area] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(0),
        Constraint::Length(2),
    ])
    .areas(frame.area());

    // Render header
    let header = header(app);
    frame.render_widget(header, header_area);

    // Render footer
    let footer = footer(app.screen(), app);
    frame.render_widget(footer, footer_area);

    // Render content
    match app.screen() {
        Screen::Main => main_draw(frame, main_area, app),
        Screen::Control => control_draw(frame, main_area, app),
        Screen::Database => database_draw(frame, main_area, app),
        Screen::Settings => settings_draw(frame, main_area, app),
        Screen::Stats => stats_draw(frame, main_area, app),
    };
}
