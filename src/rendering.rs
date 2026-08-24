// src/rendering.rs

use super::app::{App, MainSelection, Screen, SettingsSelection};
use super::math::{coggan_pwr_model, olt_hr_model};

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

// ====================================
// --------- Helper Functions ---------
// ====================================

// Footer rendering function
fn footer(current: Screen, app: &App) -> Paragraph<'_> {
    let style = Style::default()
        .add_modifier(Modifier::BOLD)
        .fg(Color::Green);
    let sep = Span::styled(" | ", style.fg(Color::Gray));

    // Initialize un-highlighted
    let mainspan = Span::styled(" Main page", style);
    let conspan = Span::styled(" Control panel", style);
    let dbspan = Span::styled(" Database", style);
    let setspan = Span::styled(" Settings", style);
    let userspan = Span::styled(app.user(), style);
    let connspan = Span::styled(app.connection(), style);
    let statsspan = Span::styled(" Stats", style);

    let current_page: Span<'_>;

    // Highlight current screen
    match current {
        Screen::Main => current_page = mainspan,
        Screen::Control => current_page = conspan,
        Screen::Database => current_page = dbspan,
        Screen::Settings => current_page = setspan,
        Screen::Stats => current_page = statsspan,
    }

    let footerspan = vec![current_page, sep.clone(), userspan, sep.clone(), connspan];
    let footerblock = Block::default()
        .borders(Borders::LEFT | Borders::RIGHT | Borders::TOP)
        .fg(Color::DarkGray)
        .border_type(BorderType::Rounded);
    let footerline = Line::from(footerspan);

    Paragraph::new(footerline).block(footerblock)
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

fn control_draw(frame: &mut Frame, area: Rect, app: &App) {
    // Data
    let livedata = app.livedata();
    let userdata = app.userdata();
    let workout_data = app.workout_data();

    let _selected = app.selections().control();

    // Colors
    let dstl = Style::default();
    let darkgray = dstl.fg(Color::DarkGray);
    let white = dstl.fg(Color::White);
    let boldwhite = white.add_modifier(Modifier::BOLD);
    let lightred = dstl.fg(Color::LightRed);
    let yellow = dstl.fg(Color::Yellow);
    let boldyellow = yellow.add_modifier(Modifier::BOLD);
    let lightblue = dstl.fg(Color::LightBlue);
    let boldlightblue = lightblue.add_modifier(Modifier::BOLD);
    let lightgreen = dstl.fg(Color::LightGreen);

    // Core Interface Layout Split
    let [hud_area, main_area] = Layout::vertical([
        Constraint::Length(6), // Extended HUD (Allows larger stat fonts)
        Constraint::Min(10),   // Split Workout Tracking Panels
    ])
    .areas(area);

    // Hi-Vis HUD Layout
    let [pwr_rect, hr_rect, rpm_rect, kmh_rect] = Layout::horizontal([
        Constraint::Percentage(25),
        Constraint::Percentage(25),
        Constraint::Percentage(25),
        Constraint::Percentage(25),
    ])
    .areas(hud_area);

    // Power Helpers
    let ftp = userdata.stats.ftp;
    let (pwrcolor, pwrzone, ltpwrprcnt) = coggan_pwr_model(livedata.crnt_pwr, ftp);

    // Power HUD
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled("Z", boldwhite),
                Span::styled(format!("{}", pwrzone), pwrcolor),
                Span::styled(" | ", boldwhite),
                Span::styled(
                    format!("{} ", livedata.crnt_pwr),
                    dstl.fg(pwrcolor).add_modifier(Modifier::BOLD),
                ),
                Span::styled("W", boldwhite),
                Span::styled(" | ", boldwhite),
                Span::styled(format!("{:>5.1} ", ltpwrprcnt), pwrcolor),
                Span::styled("% FTP", boldwhite),
            ])
            .alignment(Alignment::Center),
            Line::from(""),
            Line::from(vec![
                Span::styled("20m: ", darkgray),
                Span::styled(
                    format!("{}  ", livedata.avg_20min_pwr),
                    coggan_pwr_model(livedata.avg_20min_pwr, ftp).0,
                ),
                Span::styled("10m: ", darkgray),
                Span::styled(
                    format!("{}  ", livedata.avg_10min_pwr),
                    coggan_pwr_model(livedata.avg_10min_pwr, ftp).0,
                ),
                Span::styled("5m: ", darkgray),
                Span::styled(
                    format!("{}  ", livedata.avg_5min_pwr),
                    coggan_pwr_model(livedata.avg_5min_pwr, ftp).0,
                ),
            ])
            .alignment(Alignment::Center),
            Line::from(vec![
                Span::styled("TRGT: ", boldwhite),
                Span::styled(format!("{}  ", livedata.target_pwr), boldwhite),
                Span::styled("AVG: ", boldwhite),
                Span::styled(format!("{} ", livedata.avg_pwr), boldwhite),
                Span::styled("MAX: ", boldwhite),
                Span::styled(format!("{}  ", livedata.max_pwr), boldwhite),
            ])
            .alignment(Alignment::Center),
        ])
        .block(
            Block::new()
                .borders(Borders::ALL)
                .title(" Power Output ")
                .fg(pwrcolor),
        ),
        pwr_rect,
    );

    // HR Helpers
    let maxhr = userdata.stats.maxhr;
    let (hrcolor, hrzone, hrmaxprcnt) = olt_hr_model(livedata.crnt_hr, maxhr);
    let hrclrstyle = dstl.fg(hrcolor);
    let hrblock = Block::new()
        .borders(Borders::ALL)
        .title(" Heart Rate ")
        .fg(hrcolor);

    // Heart Rate HUD
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(
                    format!(" {} ", livedata.crnt_hr),
                    hrclrstyle.add_modifier(Modifier::BOLD),
                ),
                Span::styled("BPM", boldwhite),
            ])
            .alignment(Alignment::Center),
            Line::from(""),
            Line::from(vec![
                Span::styled("  ZONE ", boldwhite),
                Span::styled(format!("{}", hrzone), hrclrstyle),
                Span::styled(" | ", boldwhite),
                Span::styled(format!("{:.0}", hrmaxprcnt), hrclrstyle),
                Span::styled("% MAX", boldwhite),
            ])
            .alignment(Alignment::Center),
            Line::from(vec![
                Span::styled("TRGTZ: ", boldwhite),
                Span::styled(format!("{}  ", livedata.target_hrz), boldwhite),
                Span::styled("AVG: ", boldwhite),
                Span::styled(
                    format!("{}  ", livedata.avg_hr),
                    olt_hr_model(livedata.avg_hr, maxhr).0,
                ),
                Span::styled("MAX: ", boldwhite),
                Span::styled(
                    format!("{}  ", livedata.max_hr),
                    olt_hr_model(livedata.max_hr, maxhr).0,
                ),
            ])
            .alignment(Alignment::Center),
        ])
        .block(hrblock),
        hr_rect,
    );

    // Cadence HUD
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(format!(" {} ", livedata.crnt_rpm), boldwhite),
                Span::styled("RPM", darkgray),
            ])
            .alignment(Alignment::Center),
            Line::from(""),
            Line::from(vec![
                Span::styled("TRGT: ", boldwhite),
                Span::styled(format!("{}  ", livedata.target_rpm), boldlightblue),
                Span::styled("AVG: ", darkgray),
                Span::styled(format!("{}  ", livedata.avg_rpm), boldlightblue),
                Span::styled("MAX: ", darkgray),
                Span::styled(format!("{}  ", livedata.max_rpm), boldlightblue),
            ])
            .alignment(Alignment::Center),
        ])
        .block(Block::new().borders(Borders::ALL).title(" Cadence ")),
        rpm_rect,
    );

    // Speed HUD
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(
                    format!(" {:.1} ", livedata.crnt_vel),
                    dstl.fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ),
                Span::styled("KM/H", darkgray),
            ])
            .alignment(Alignment::Center),
            Line::from(""),
            Line::from(vec![
                Span::styled("TRGT: ", darkgray),
                Span::styled(format!("{} ", livedata.target_vel), boldlightblue),
                Span::styled("AVG: ", darkgray),
                Span::styled(format!("{} ", livedata.avg_vel), boldlightblue),
                Span::styled("MAX: ", darkgray),
                Span::styled(format!("{}  ", livedata.max_vel), boldlightblue),
            ])
            .alignment(Alignment::Center),
        ])
        .block(
            Block::new()
                .borders(Borders::ALL)
                .title(" Speed ")
                .fg(Color::Cyan),
        ),
        kmh_rect,
    );

    // Main Split
    let [left_panel, right_panel] =
        Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)])
            .areas(main_area);

    // Left Panel: Real-time Ride Progress Metrics
    let workout_metrics = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  ELAPSED:      ", darkgray),
            Span::styled(
                format!(
                    "{}m {}s",
                    livedata.elapsed_secs / 60,
                    livedata.elapsed_secs % 60
                ),
                boldwhite,
            ),
        ]),
        Line::from(vec![
            Span::styled("  REMAINING:  ", darkgray),
            Span::styled(
                format!(
                    "{}m {}s",
                    workout_data.duration / 60,
                    workout_data.duration % 60
                ),
                boldwhite,
            ),
        ]),
        Line::from(vec![
            Span::styled("  DISTANCE:  ", darkgray),
            Span::styled(format!("{} km", livedata.elapsed_distance), boldwhite),
            Span::styled(" / ", boldwhite),
            Span::styled(format!("{} km", workout_data.total_distance), boldwhite),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ELEV. GAIN:  ", darkgray),
            Span::styled(format!("+{} m", livedata.egain), lightgreen),
        ]),
        Line::from(vec![
            Span::styled("  ELEV. LOSS:  ", darkgray),
            Span::styled(format!("-{} m", livedata.eloss), lightred),
        ]),
        Line::from(vec![
            Span::styled("  GRADIENT:    ", darkgray),
            Span::styled(format!("{}%", livedata.grad), boldwhite),
        ]),
        Line::from(vec![
            Span::styled("  ALTITUDE:    ", darkgray),
            Span::styled(format!("{} m", livedata.alti), boldwhite),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  NP:        ", darkgray),
            Span::styled(format!("{} W", livedata.normalized_pwr), boldyellow),
        ]),
        Line::from(vec![
            Span::styled("  IF:        ", darkgray),
            Span::styled(format!("{:.2}", livedata.ifac), boldyellow),
        ]),
        Line::from(vec![
            Span::styled("  TSS:       ", darkgray),
            Span::styled(format!("{:.2}", livedata.tss), boldyellow),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ENERGY:    ", darkgray),
            Span::styled(format!("{}", livedata.kj), boldyellow),
            Span::styled(" kJ", boldwhite),
        ]),
        Line::from(vec![
            Span::styled("  CALORIES:  ", darkgray),
            Span::styled(format!("{} ", livedata.calories), lightred),
            Span::styled("kcal", boldwhite),
        ]),
    ];

    // Side Panel
    frame.render_widget(
        Paragraph::new(workout_metrics).block(
            Block::new()
                .borders(Borders::ALL)
                .title(" Session Metrics "),
        ),
        left_panel,
    );

    // Virtual road
    let perspective_road = vec![
        Line::from(""),
        Line::from(Span::styled("                     /  *  \\", darkgray)),
        Line::from(Span::styled("                    /   *   \\", darkgray)),
        Line::from(Span::styled("                   /    *    \\", darkgray)),
        Line::from(Span::styled("                  /     *     \\", darkgray)),
        Line::from(Span::styled("                 /      *      \\", darkgray)),
        Line::from(vec![
            Span::styled("                /       ", darkgray),
            Span::styled("▲ YOU", boldyellow),
            Span::styled("   \\", darkgray),
        ]),
        Line::from(Span::styled(
            "               /        *        \\",
            darkgray,
        )),
        Line::from(Span::styled(
            "              /         *         \\",
            darkgray,
        )),
        Line::from(vec![Span::styled(
            "             /          *          \\",
            darkgray,
        )]),
        Line::from(Span::styled(
            "            /           *           \\",
            darkgray,
        )),
        Line::from(Span::styled(
            "___________/____________*____________\\___________",
            darkgray,
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  📍 Location: Watopia Epic KOM Pass (Sector 3)",
            darkgray,
        )),
    ];

    frame.render_widget(
        Paragraph::new(perspective_road).block(
            Block::new()
                .borders(Borders::ALL)
                .title(" Live Route Simulator "),
        ),
        right_panel,
    );
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
        SettingsSelection::Bluetooth => Paragraph::new(format!(
            "Bluetooth Devices\n-----------------\n[*] {}",
            app.devices()
        )),
        SettingsSelection::System => Paragraph::new(format!(
            "System Information\n------------------\nVersion: {}",
            app.version()
        )),
        SettingsSelection::User => Paragraph::new(format!(
            "User Settings\n--------------\n[ ] Dark Mode\n[ ] High Contrast"
        )),
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
    let [main_area, footer_area] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(2)]).areas(frame.area());

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
