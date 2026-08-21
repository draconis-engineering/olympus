// src/rendering.rs

use super::app::{
    App, ControlSelection, DatabaseSelection, MainSelection, Screen, Selections, SettingsSelection,
};

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

// ====================================
// --------- Helper Functions ---------
// ====================================

// Footer rendering function
fn footer_spans(current: Screen) -> Vec<Span<'static>> {
    // Default style
    let default_style = Style::default();
    let quit_style = Style::default().fg(Color::Red);
    let sep = Span::styled(" | ", default_style);

    // Initialize un-highlighted
    let mut mainspan = Span::styled("[ M ] Main page", default_style);
    let mut conspan = Span::styled("[ C ] Control panel", default_style);
    let mut dbspan = Span::styled("[ D ] Database", default_style);
    let mut setspan = Span::styled("[ S ] Settings", default_style);
    let quitspan = Span::styled("[ Q ] Quit", quit_style);

    // Highlighted style
    let active_style = Style::default()
        .add_modifier(Modifier::BOLD)
        .fg(Color::Green);

    // Highlight current screen
    match current {
        Screen::Main => mainspan = mainspan.style(active_style),
        Screen::Control => conspan = conspan.style(active_style),
        Screen::Database => dbspan = dbspan.style(active_style),
        Screen::Settings => setspan = setspan.style(active_style),
    }

    vec![
        mainspan,
        sep.clone(),
        conspan,
        sep.clone(),
        dbspan,
        sep.clone(),
        setspan,
        sep.clone(),
        quitspan,
    ]
}

// Get color from HR zone | Add softcoded HRZs or percentage based zones
fn hr2color(hr: u16) -> (Color, u16) {
    match hr {
        0..=113 => (Color::White, 0),
        114..=149 => (Color::Gray, 1),
        150..=170 => (Color::LightBlue, 2),
        171..=180 => (Color::Green, 3),
        181..=191 => (Color::Yellow, 4),
        192..=220 => (Color::Red, 5),
        _ => (Color::White, 0),
    }
}

// Convert power + lactate threshold power to color for rendering based on power zones and Dr Andrew Coggans Model
fn pwr2color(pwr: u16, ltpwr: u16) -> (Color, u16) {
    // Color, Zone, Zone description
    let ltpwr_percentage = (pwr as f32 / ltpwr as f32) * 100.0;
    match ltpwr_percentage.round() {
        0.0..=54.0 => (Color::LightBlue, 1),
        55.0..=75.0 => (Color::Blue, 2),
        76.0..=90.0 => (Color::Green, 3),
        91.0..=105.0 => (Color::Yellow, 4),
        106.0..=120.0 => (Color::Rgb(255, 128, 0), 5), // Orange
        121.0..=150.0 => (Color::Red, 6),
        151.0..=1000.0 => (Color::Rgb(255, 192, 203), 7), // Pink
        _ => (Color::White, 0),
    }
}

// ====================================
// --- Page-Specific Draw Functions ---
// ====================================

fn main_draw(frame: &mut Frame, area: Rect, selected: &MainSelection, app: &App) {
    let livedata = app.livedata();
    let userdata = app.userdata();

    // Split screen vertically into Header, Main Dashboard, and Footer Hints
    let [hud, main, footer] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Top HUD Header
            Constraint::Min(12),   // Game Title & Navigation
            Constraint::Length(3), // Hotkey Action Hints
        ])
        .areas(area);

    // --- TOP HUD HEADER ---
    let [pwr, rpm, bpm, kmh] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .areas(hud);

    // --- HELPERS FOR COLORS & ZONES ---
    let (pwrcolor, pwrzone) = pwr2color(livedata.pwr, userdata.ltpwr);
    let (hrcolor, hrzone) = hr2color(livedata.hr);

    frame.render_widget(
        Paragraph::new(format!(" ⚡ {} W | Zone {}", livedata.pwr, pwrzone)).block(
            Block::new()
                .borders(Borders::ALL)
                .fg(pwrcolor)
                .title(" Power "),
        ),
        pwr,
    );
    frame.render_widget(
        Paragraph::new(format!(" 🔄 {} RPM", livedata.rpm))
            .block(Block::new().borders(Borders::ALL).title(" Cadence ")),
        rpm,
    );
    frame.render_widget(
        Paragraph::new(format!(" ❤️ {} BPM | Zone {}", livedata.hr, hrzone)).block(
            Block::new()
                .borders(Borders::ALL)
                .fg(hrcolor)
                .title(" Heart Rate "),
        ),
        bpm,
    );
    frame.render_widget(
        Paragraph::new(format!(" 💨 {:.1} KM/H", livedata.vel))
            .block(Block::new().borders(Borders::ALL).title(" Speed ")),
        kmh,
    );

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
        .areas(main);

    // Multi-line slant ASCII title
    let ascii_logo = vec![
        Line::from(Span::styled(
            "  ____  _ __     ____  __ ____  _   _ ____  ",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(Span::styled(
            " / __ \\| |\\ \\   / /  \\/  |  _ \\| | | / ___| ",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(Span::styled(
            "| |  | | | \\ \\ / /| |\\/| | |_) | | | \\___ \\ ",
            Style::default().fg(Color::LightCyan),
        )),
        Line::from(Span::styled(
            "| |__| | |__| \\ / | |  | |  __/| |_| |___) |",
            Style::default().fg(Color::Blue),
        )),
        Line::from(Span::styled(
            " \\____/|_____|_|  |_|  |_|_|    \\___/|____/ ",
            Style::default().fg(Color::LightBlue),
        )),
        Line::from(Span::styled(
            "   -- FREE OPEN-SOURCE TURBO TRAINER - BY DRACONIS --   ",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    frame.render_widget(
        Paragraph::new(ascii_logo).alignment(Alignment::Center),
        ascii,
    );

    // Main menu selection text
    let mut menu_text = vec![
        Line::from(Span::styled(
            "     START NEW RIDE     ",
            Style::default().fg(Color::Gray),
        )),
        Line::from(Span::styled(
            " SELECT ROUTE (WATOPIA) ",
            Style::default().fg(Color::Gray),
        )),
        Line::from(Span::styled(
            "     PAST  WORKOUTS     ",
            Style::default().fg(Color::Gray),
        )),
        Line::from(Span::styled(
            "   SETTINGS & SENSORS   ",
            Style::default().fg(Color::Gray),
        )),
    ];

    // Highlight the selected menu item
    match selected {
        MainSelection::Main => menu_text[0].spans[0].style = Style::default().fg(Color::Green),
        MainSelection::Route => menu_text[1].spans[0].style = Style::default().fg(Color::Green),
        MainSelection::Workouts => menu_text[2].spans[0].style = Style::default().fg(Color::Green),
        MainSelection::Settings => menu_text[3].spans[0].style = Style::default().fg(Color::Green),
    }

    frame.render_widget(
        Paragraph::new(menu_text).alignment(Alignment::Center),
        navopt,
    );

    // --- NEW: FOOTER BUTTON HINTS ---
    let hint_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(footer);

    // Render 3 cleanly aligned gamepad-style text blocks at the bottom row
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" Press ", Style::default().fg(Color::Gray)),
            Span::styled(
                "[Enter]",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" to launch session", Style::default().fg(Color::Gray)),
        ]))
        .alignment(Alignment::Center)
        .block(Block::new().borders(Borders::TOP).fg(Color::DarkGray)),
        hint_layout[0],
    );

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" Use ", Style::default().fg(Color::Gray)),
            Span::styled(
                "[↑/↓]",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" to change selection", Style::default().fg(Color::Gray)),
        ]))
        .alignment(Alignment::Center)
        .block(Block::new().borders(Borders::TOP).fg(Color::DarkGray)),
        hint_layout[1],
    );

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" Press ", Style::default().fg(Color::Gray)),
            Span::styled(
                "[Esc]",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" to exit app", Style::default().fg(Color::Gray)),
        ]))
        .alignment(Alignment::Center)
        .block(Block::new().borders(Borders::TOP).fg(Color::DarkGray)),
        hint_layout[2],
    );
}

fn control_draw(frame: &mut Frame, area: Rect, selected: &ControlSelection, app: &App) {
    let livedata = app.livedata();
    let userdata = app.userdata();

    // 1. Core Interface Layout Split
    let [hud_area, main_area, footer_area] = Layout::vertical([
        Constraint::Length(5), // Extended HUD (Allows larger stat fonts)
        Constraint::Min(10),   // Split Workout Tracking Panels
        Constraint::Length(3), // In-ride Hotkey Actions
    ])
    .areas(area);

    // 2. High-Visibility HUD Layout
    let [pwr_rect, hr_rect, rpm_rect, kmh_rect] = Layout::horizontal([
        Constraint::Percentage(25),
        Constraint::Percentage(25),
        Constraint::Percentage(25),
        Constraint::Percentage(25),
    ])
    .areas(hud_area);

    let (pwrcolor, pwrzone) = pwr2color(livedata.pwr, userdata.ltpwr);
    let (hrcolor, hrzone) = hr2color(livedata.hr);
    let defaultstyle = Style::default().fg(Color::DarkGray);

    // Bigger, padded layout lines for easy reading while sweating
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(""), // Vertical pad
            Line::from(vec![
                Span::styled(
                    format!(" ⚡ {} ", livedata.pwr),
                    Style::default().fg(pwrcolor).add_modifier(Modifier::BOLD),
                ),
                Span::styled("W", defaultstyle),
            ])
            .alignment(Alignment::Center),
            Line::from(Span::styled(
                format!("ZONE {}", pwrzone),
                Style::default().fg(pwrcolor),
            ))
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

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    format!(" ❤️ {} ", livedata.hr),
                    Style::default().fg(hrcolor).add_modifier(Modifier::BOLD),
                ),
                Span::styled("BPM", defaultstyle),
            ])
            .alignment(Alignment::Center),
            Line::from(Span::styled(
                format!("ZONE {}", hrzone),
                Style::default().fg(hrcolor),
            ))
            .alignment(Alignment::Center),
        ])
        .block(
            Block::new()
                .borders(Borders::ALL)
                .title(" Heart Rate ")
                .fg(hrcolor),
        ),
        hr_rect,
    );

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    format!(" 🔄 {} ", livedata.rpm),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("RPM", defaultstyle),
            ])
            .alignment(Alignment::Center),
        ])
        .block(Block::new().borders(Borders::ALL).title(" Cadence ")),
        rpm_rect,
    );

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    format!(" 💨 {:.1} ", livedata.vel),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("KM/H", defaultstyle),
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

    // 3. Main Workspace Split (Left: Session Metrics, Right: Visual Simulation Road)
    let [left_panel, right_panel] =
        Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)])
            .areas(main_area);

    // Left Panel: Real-time Ride Progress Metrics
    let workout_metrics = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  ⏱  ELAPSED TIME:   ", defaultstyle),
            Span::styled(
                "00:42:18",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  🏁 DISTANCE:       ", defaultstyle),
            Span::styled(
                "21.4 km",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  ⛰  ELEVATION GAIN: ", defaultstyle),
            Span::styled("+312 m", Style::default().fg(Color::LightGreen)),
        ]),
        Line::from(vec![
            Span::styled("  🔥 CALORIES:       ", defaultstyle),
            Span::styled("542 kcal", Style::default().fg(Color::LightRed)),
        ]),
        Line::from(""),
        Line::from(Span::styled("  --- TARGET INTERVAL ---", defaultstyle)),
        Line::from(vec![
            Span::styled("  🎯 CURRENT GOAL:   ", defaultstyle),
            Span::styled("240W for 5 mins", Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("  ⏳ REMAINING:      ", defaultstyle),
            Span::styled(
                "02:15",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];

    frame.render_widget(
        Paragraph::new(workout_metrics).block(
            Block::new()
                .borders(Borders::ALL)
                .title(" Session Metrics "),
        ),
        left_panel,
    );

    // Right Panel: Text-based "Virtual Virtual World" Ride Simulator View
    let perspective_road = vec![
        Line::from(""),
        Line::from(Span::styled(
            "                     /  *  \\                     ",
            defaultstyle,
        )),
        Line::from(Span::styled(
            "                    /   *   \\                    ",
            defaultstyle,
        )),
        Line::from(Span::styled(
            "                   /    *    \\                   ",
            defaultstyle,
        )),
        Line::from(vec![
            Span::styled("  [Rider A (+0:04)] ", Style::default().fg(Color::Gray)),
            Span::styled("/     *     \\", defaultstyle),
        ]),
        Line::from(Span::styled(
            "                 /      *      \\                 ",
            defaultstyle,
        )),
        Line::from(vec![
            Span::styled("                /       ", defaultstyle),
            Span::styled(
                "▲ YOU",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("       \\                ", defaultstyle),
        ]),
        Line::from(Span::styled(
            "              /         *         \\              ",
            defaultstyle,
        )),
        Line::from(vec![
            Span::styled("             /          *          \\   ", defaultstyle),
            Span::styled("[Gradient: 6.5%]", Style::default().fg(Color::LightRed)),
        ]),
        Line::from(Span::styled(
            "___________/____________*____________\\___________",
            defaultstyle,
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  📍 Location: Watopia Epic KOM Pass (Sector 3)",
            defaultstyle,
        )),
    ];

    frame.render_widget(
        Paragraph::new(perspective_road).block(
            Block::new()
                .borders(Borders::ALL)
                .title(" Live Route Simulator (3D Text View) "),
        ),
        right_panel,
    );

    // 4. Ride Action Hotkeys Footer
    let [key1, key2, key3] = Layout::horizontal([
        Constraint::Percentage(33),
        Constraint::Percentage(34),
        Constraint::Percentage(33),
    ])
    .areas(footer_area);

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                " [Space] ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Pause Ride", Style::default().fg(Color::Gray)),
        ]))
        .alignment(Alignment::Center)
        .block(Block::new().borders(Borders::TOP).fg(Color::DarkGray)),
        key1,
    );

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                " [M] ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Toggle Map Mode", Style::default().fg(Color::Gray)),
        ]))
        .alignment(Alignment::Center)
        .block(Block::new().borders(Borders::TOP).fg(Color::DarkGray)),
        key2,
    );

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                " [Esc] ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::styled("End & Save Workout", Style::default().fg(Color::Gray)),
        ]))
        .alignment(Alignment::Center)
        .block(Block::new().borders(Borders::TOP).fg(Color::DarkGray)),
        key3,
    );
}

fn database_draw(frame: &mut Frame, area: Rect, selected: &DatabaseSelection, app: &App) {
    // Colors
    let gray = Style::default().fg(Color::Gray);
    let dark_gray = Style::default().fg(Color::DarkGray);
    let white = Style::default().fg(Color::White);

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
        Span::styled(" Watopia", Style::default().fg(Color::White)),
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
        Line::from(Span::styled(
            " \"SST Short\" Profile Preview:",
            Style::default().fg(Color::Gray),
        )),
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
            Span::styled(
                "Gorilla Intervals        ",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
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
            Span::styled(
                "SST Short (Active Plan) ",
                Style::default().fg(Color::White),
            ),
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

fn settings_draw(frame: &mut Frame, area: Rect, selected: &SettingsSelection, app: &App) {
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
    let [general, appearance, bluetooth, system, user, etc] = Layout::vertical([
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

// ====================================
// --- Drawing Multiplexer Function ---
// ====================================

pub fn draw(frame: &mut Frame, app: &App, selections: &Selections) {
    // Properly split the screen area into two horizontal sections
    let [content_area, footer_area] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(frame.area());

    // Render footer
    let footer = Paragraph::new(Line::from(footer_spans(app.screen())));
    frame.render_widget(footer, footer_area);

    // Render content
    match app.screen() {
        Screen::Main => {
            main_draw(frame, content_area, selections.main(), app);
        }
        Screen::Control => {
            control_draw(frame, content_area, selections.control(), app);
        }
        Screen::Database => {
            database_draw(frame, content_area, selections.database(), app);
        }
        Screen::Settings => {
            settings_draw(frame, content_area, selections.settings(), app);
        }
    };
}
