// src/rendering.rs

use super::app::{App, Screen, Selections, SettingsSelection};

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::Color::White;
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

// ====================================
// --------- Helper Functions ---------
// ====================================

// Footer rendering function
fn footer_spans(current: Screen) -> Vec<Span<'static>> {
    // Default style
    let default_style = Style::default();
    let quit_style = Style::default().fg(Color::Red);

    // Initialize un-highlighted
    let mut mainspan = Span::styled("[ M ] Main page | ", default_style);
    let mut conspan = Span::styled("[ C ] Control panel | ", default_style);
    let mut dbspan = Span::styled("[ D ] Database | ", default_style);
    let mut setspan = Span::styled("[ S ] Settings | ", default_style);
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

    vec![mainspan, conspan, dbspan, setspan, quitspan]
}

// Get color from HR zone
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

fn main_draw(frame: &mut Frame, area: Rect, app: &App) {
    let data = app.data();

    // Split screen vertically into Header, Main Visual, and Footer status bar
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Top HUD Header
            Constraint::Min(10),   // Main Virtual Road / Chart Area
        ])
        .split(area);

    // Split the Top HUD horizontally into 4 quick-glance segments
    let hud_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(main_layout[0]);

    // Render Top HUD Stats
    frame.render_widget(
        Paragraph::new(format!(" ⚡ {} W", data.pwr)).block(Block::new().borders(Borders::ALL)),
        hud_layout[0],
    );
    frame.render_widget(
        Paragraph::new(format!(" 🔄 {} RPM", data.cadence))
            .block(Block::new().borders(Borders::ALL)),
        hud_layout[1],
    );
    frame.render_widget(
        Paragraph::new(format!(" ❤️ {} BPM", data.hr)).block(Block::new().borders(Borders::ALL)),
        hud_layout[2],
    );
    frame.render_widget(
        Paragraph::new(format!(" 💨 {:.1} KM/H", data.speed))
            .block(Block::new().borders(Borders::ALL)),
        hud_layout[3],
    );

    // Centered Dashboard Content (Zwift Visual Mockup)
    // You could replace this later with a Canvas, Sparkline, or Chart widget
    let center_block = Block::new()
        .title(" Virtual Route View ")
        .borders(Borders::ALL);

    let mock_visual = Paragraph::new("\n\n   ▲  [You] Climbing Watopia Epic KOM (1.2km remaining at 7%)\n   ▲  Rider B (+0:14)\n   ▼  Rider C (-0:05)")
        .block(center_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(mock_visual, main_layout[1]);
}

fn control_draw(frame: &mut Frame, area: Rect, app: &App) {
    let data = app.data();

    // Layout - split into 30% HUD and 70% main area
    let [hudrect, mainrect] =
        Layout::vertical([Constraint::Percentage(30), Constraint::Percentage(70)]).areas(area);

    let [pwrrect, hrrect, rpmrect, kmhrect] = Layout::horizontal([
        Constraint::Percentage(25),
        Constraint::Percentage(25),
        Constraint::Percentage(25),
        Constraint::Percentage(25),
    ])
    .areas(hudrect);

    // HUD data displays
    let (pwrcolor, pwrzone) = pwr2color(data.pwr, data.ltpwr);
    let pwrblck = Paragraph::new(Line::from(vec![Span::styled(
        format!("{} Watts | Zone {}", data.pwr, pwrzone),
        Style::default(),
    )]))
    .block(
        Block::new()
            .borders(Borders::ALL)
            .title(" Power ")
            .fg(pwrcolor),
    );

    let (hrcolor, hrzone) = hr2color(data.hr);
    let hrblck = Paragraph::new(Line::from(vec![Span::styled(
        format!("{} BPM | Zone {}", data.hr, hrzone),
        Style::default(),
    )]))
    .block(
        Block::new()
            .borders(Borders::ALL)
            .title(" Heart Rate ")
            .fg(hrcolor),
    );

    let rpmblck = Paragraph::new(Line::from(vec![Span::styled(
        format!("{} RPM", data.cadence),
        Style::default(),
    )]))
    .block(Block::new().borders(Borders::ALL).title(" Cadence "));

    let kmhblck = Paragraph::new(Line::from(vec![Span::styled(
        format!("{} km/h", data.speed),
        Style::default(),
    )]))
    .block(
        Block::new()
            .borders(Borders::ALL)
            .title(" Speed ")
            .fg(White),
    );

    frame.render_widget(pwrblck, pwrrect);
    frame.render_widget(hrblck, hrrect);
    frame.render_widget(rpmblck, rpmrect);
    frame.render_widget(kmhblck, kmhrect);

    // Graphs - placed on the 70% main area, displaying various graphs
    let [graphs, placeholder] =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .areas(mainrect);

    let [altirect, speedrect] =
        Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(graphs);

    let phblck = Paragraph::new(Line::from(vec![Span::styled(
        "Placeholder",
        Style::default(),
    )]))
    .block(Block::new().borders(Borders::ALL));

    //frame.render_widget(altichart, altirect);
    //frame.render_widget(speedchart, speedrect);
    frame.render_widget(phblck, placeholder);
}

fn database_draw(frame: &mut Frame, area: Rect, app: &App) {
    let data = app.data();

    let [sidebar_area, list_area] =
        Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)]).areas(area);

    let [searchbar_area, filters_area, etc_area, buttons_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(8),
        Constraint::Min(0),
        Constraint::Length(3),
    ])
    .areas(sidebar_area);

    let search = Block::new().borders(Borders::ALL).title(" Search ");
    let filters = Block::new().borders(Borders::ALL).title(" Filters ");
    let buttons = Block::new().borders(Borders::ALL);
    let etc = Block::new().borders(Borders::ALL);

    let list = Block::new()
        .borders(Borders::ALL)
        .title(" Workouts ")
        .fg(Color::DarkGray);

    frame.render_widget(filters, filters_area);
    frame.render_widget(search, searchbar_area);
    frame.render_widget(buttons, buttons_area);
    frame.render_widget(etc, etc_area);
    frame.render_widget(list, list_area);
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
            main_draw(frame, content_area, app);
        }
        Screen::Control => {
            control_draw(frame, content_area, app);
        }
        Screen::Database => {
            database_draw(frame, content_area, app);
        }
        Screen::Settings => {
            settings_draw(frame, content_area, selections.settings(), app);
        }
    };
}
