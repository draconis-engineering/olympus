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
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
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

fn control_draw(frame: &mut Frame, area: Rect, app: &App) {
    let livedata = app.livedata();
    let userdata = app.userdata();
    let userstats = &userdata.stats;

    let [metrics, stats] =
        Layout::vertical([Constraint::Percentage(65), Constraint::Percentage(35)]).areas(area);

    let [pwrrect, hr_rpm_vel] =
        Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)]).areas(metrics);

    let [hrrect, rpm_vel] =
        Layout::vertical([Constraint::Percentage(55), Constraint::Percentage(45)])
            .areas(hr_rpm_vel);

    let [rpmrect, velrect] =
        Layout::horizontal([Constraint::Percentage(45), Constraint::Percentage(55)]).areas(rpm_vel);

    let [pwrzrect, statrect, intvlrect, sysrect] = Layout::horizontal([
        Constraint::Percentage(40),
        Constraint::Percentage(20),
        Constraint::Percentage(20),
        Constraint::Percentage(20),
    ])
    .areas(stats);

    // POWER GRID

    let pwrblock = Block::default()
        .title(" POWER ")
        .borders(Borders::ALL)
        .fg(Color::Yellow)
        .border_type(BorderType::Rounded);

    let inner = pwrblock.inner(pwrrect);

    let [zone, pwrtext, pwrgraph, pwrstats, pwrtarget] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(6),
        Constraint::Fill(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(inner);

    let pwrzline = Line::from(vec![
        Span::from(" ZONE "),
        Span::from(format!(
            "{}",
            coggan_pwr_model(livedata.crnt_pwr, userstats.ftp)
        ))
        .fg(Color::Yellow),
        Span::from(" / ").fg(Color::DarkGray),
    ]);

    let pwrtrgtline = Line::from(vec![
        Span::from(" TARGET ").fg(Color::DarkGray),
        Span::from(format!("{}", livedata.target_pwr)).fg(Color::Yellow),
        Span::from(" W").fg(Color::DarkGray),
    ]);

    let hrblock = Block::default()
        .title(" HEART RATE ")
        .borders(Borders::ALL)
        .fg(Color::Red)
        .border_type(BorderType::Rounded);

    let rpmblock = Block::default()
        .title(" CADENCE ")
        .borders(Borders::ALL)
        .fg(Color::LightBlue)
        .border_type(BorderType::Rounded);

    let velblock = Block::default()
        .title(" SPEED ")
        .borders(Borders::ALL)
        .fg(Color::Green)
        .border_type(BorderType::Rounded);

    let pwrzblock = Block::default()
        .title(" POWER ZONES ")
        .borders(Borders::ALL)
        .fg(Color::DarkGray)
        .border_type(BorderType::Rounded);

    let pwrztextrect = pwrzblock.inner(pwrzrect);

    let pwrztext = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![Span::from("  Z1  ").fg(Color::DarkGray)]),
        Line::from(vec![Span::from("  Z2  ").fg(Color::DarkGray)]),
        Line::from(vec![Span::from("  Z3  ").fg(Color::DarkGray)]),
        Line::from(vec![Span::from("  Z4  ").fg(Color::DarkGray)]),
        Line::from(vec![Span::from("  Z5  ").fg(Color::DarkGray)]),
        Line::from(vec![Span::from("  Z6  ").fg(Color::DarkGray)]),
        Line::from(vec![Span::from("  Z7  ").fg(Color::DarkGray)]),
    ]);

    let statblock = Block::default()
        .title(" RIDE STATS ")
        .borders(Borders::ALL)
        .fg(Color::DarkGray)
        .border_type(BorderType::Rounded);

    let statstextrect = statblock.inner(statrect);

    let statstext = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::from("  DURATION:   "),
            Span::from(format!(
                "{}:{}:{}",
                livedata.elapsed_secs / 3600,
                (livedata.elapsed_secs / 60) % 60,
                livedata.elapsed_secs % 60
            )),
        ]),
        Line::from(vec![
            Span::from("  DISTANCE:   "),
            Span::from(format!("{} km", livedata.elapsed_distance)),
        ]),
        Line::from(vec![
            Span::from("  ELEVATION:  "),
            Span::from(format!("{} m", livedata.alti)),
        ]),
        Line::from(vec![
            Span::from("  GRADIENT:   "),
            Span::from(format!("{}%", livedata.grad)),
        ]),
        Line::from(vec![
            Span::from("  CALORIES:   "),
            Span::from(format!("{} kcal", livedata.calories)),
        ]),
        Line::from(vec![
            Span::from("  TSS:        "),
            Span::from(format!("{}", livedata.tss)),
        ]),
        Line::from(vec![
            Span::from("  IF:         "),
            Span::from(format!("{}", livedata.ifac)),
        ]),
    ]);

    let intvlblock = Block::default()
        .title(" INTERVALS ")
        .borders(Borders::ALL)
        .fg(Color::DarkGray)
        .border_type(BorderType::Rounded);

    let sysblock = Block::default()
        .title(" SYSTEM ")
        .borders(Borders::ALL)
        .fg(Color::DarkGray)
        .border_type(BorderType::Rounded);

    frame.render_widget(pwrblock, pwrrect);
    frame.render_widget(pwrzline, zone);
    frame.render_widget(pwrtrgtline, pwrtarget);

    frame.render_widget(hrblock, hrrect);

    frame.render_widget(rpmblock, rpmrect);

    frame.render_widget(velblock, velrect);

    frame.render_widget(pwrzblock, pwrzrect);
    frame.render_widget(pwrztext, pwrztextrect);

    frame.render_widget(statblock, statrect);
    frame.render_widget(statstext, statstextrect);

    frame.render_widget(intvlblock, intvlrect);

    frame.render_widget(sysblock, sysrect);
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
