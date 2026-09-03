// src/rendering.rs
//
// Rendering.rs is responsible for rendering the Olympus interface and its pages.

use super::app::{App, BleUiState, DatabaseTab, RideState, Screen, SettingsField};
use super::math::{coggan_pwr_model, olt_hr_model, zone2color};
use super::nav::{MainSelection, SettingsSelection};

use chrono::Local;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Flex, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Axis, Block, BorderType, Borders, Chart, Clear, Dataset, Gauge, GraphType, Paragraph,
};
use tui_big_text::{BigText, PixelSize};

// ====================================
// --------- Helper Functions ---------
// ====================================

/// Convert the tail of a sample history into (x, y) points for a line chart,
/// keeping at most `max_points` (so the visible line stays current and
/// readable instead of compressing the whole ride into a few columns).
fn tail_points<F>(vals: &[f64], max_points: usize, map: F) -> Vec<(f64, f64)>
where
    F: Fn(f64) -> f64,
{
    let n = vals.len().min(max_points);
    let start = vals.len().saturating_sub(n);
    vals[start..]
        .iter()
        .enumerate()
        .map(|(i, &v)| (i as f64, map(v)))
        .collect()
}

/// Build a braille line chart rendered onto the given rectangle.
fn line_chart<'a>(points: &'a [(f64, f64)], color: Color, max_y: f64) -> Chart<'a> {
    let dataset = Dataset::default()
        .marker(Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(color))
        .data(points);
    let x_axis = Axis::default()
        .style(Style::default().fg(Color::DarkGray))
        .bounds([0.0, (points.len() as f64).max(1.0) - 1.0]);
    let y_axis = Axis::default()
        .style(Style::default().fg(Color::DarkGray))
        .bounds([0.0, max_y]);
    Chart::new(vec![dataset]).x_axis(x_axis).y_axis(y_axis)
}

/// Center a box that is `percent_x` x `percent_y` (as a percentage of `area`)
/// in the middle of the screen, for overlays/dialogs.
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vert: [Rect; 3] = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .flex(Flex::Center)
    .areas(area);
    let horiz: [Rect; 3] = Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .flex(Flex::Center)
    .areas(vert[1]);
    horiz[1]
}

/// Draw the "loading workout" splash overlay over the whole screen.
fn render_loading(frame: &mut Frame, area: Rect) {
    let popup = centered_rect(46, 30, area);
    frame.render_widget(Clear, popup);
    let lines = vec![
        Line::from(Span::styled(
            " OLYMPUS ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center),
        Line::from(""),
        Line::from(Span::styled(
            "Loading workout...",
            Style::default().fg(Color::Gray),
        ))
        .alignment(Alignment::Center),
        Line::from(""),
        Line::from(Span::styled(
            "Connecting trainer & setting ERG targets",
            Style::default().fg(Color::DarkGray),
        ))
        .alignment(Alignment::Center),
        Line::from(""),
        Line::from(Span::styled(
            "█▒▒▒▒▒▒▒▒",
            Style::default().fg(Color::Yellow),
        ))
        .alignment(Alignment::Center),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .alignment(Alignment::Center),
        popup,
    );
}

/// Draw the "are you sure you want to quit?" confirmation dialog.
fn render_confirm_quit(frame: &mut Frame, area: Rect) {
    let popup = centered_rect(44, 26, area);
    frame.render_widget(Clear, popup);
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Are you sure you want to quit?",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center),
        Line::from(""),
        Line::from(Span::styled(
            "Any unsaved ride data will be lost.",
            Style::default().fg(Color::DarkGray),
        ))
        .alignment(Alignment::Center),
        Line::from(""),
        Line::from(Span::styled(
            "[ Y ] Yes    [ N / Esc ] No",
            Style::default().fg(Color::Cyan),
        ))
        .alignment(Alignment::Center),
        Line::from(""),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(" Quit ")
                    .border_style(Style::default().fg(Color::Red)),
            )
            .alignment(Alignment::Center),
        popup,
    );
}

/// Draw the end-of-ride summary dialog (Save / Discard / Resume).
fn render_summary(frame: &mut Frame, app: &App, area: Rect) {
    let d = app.livedata();
    let h = d.elapsed_secs / 3600;
    let m = (d.elapsed_secs / 60) % 60;
    let s = d.elapsed_secs % 60;
    let avg_speed = if d.elapsed_secs > 0 {
        d.elapsed_distance / (d.elapsed_secs as f32 / 3600.0)
    } else {
        0.0
    };
    let popup = centered_rect(56, 62, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Ride Summary ")
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let [summary_area, hint_area] = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(3),
    ])
    .areas(inner);

    let lines = vec![
        Line::from(Span::styled(
            " Ride complete — save your session?",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Time:     ", Color::DarkGray),
            Span::raw(format!("{h:02}:{m:02}:{s:02}")),
            Span::styled("    Distance: ", Color::DarkGray),
            Span::raw(format!("{:.2} km", d.elapsed_distance)),
        ]),
        Line::from(vec![
            Span::styled("Avg Pwr:  ", Color::DarkGray),
            Span::raw(format!("{} W", d.avg_pwr)),
            Span::styled("    Max Pwr:  ", Color::DarkGray),
            Span::raw(format!("{} W", d.max_pwr)),
        ]),
        Line::from(vec![
            Span::styled("Avg HR:   ", Color::DarkGray),
            Span::raw(format!("{} bpm", d.avg_hr)),
            Span::styled("    Max HR:   ", Color::DarkGray),
            Span::raw(format!("{} bpm", d.max_hr)),
        ]),
        Line::from(vec![
            Span::styled("Avg Spd:  ", Color::DarkGray),
            Span::raw(format!("{avg_speed:.1} km/h")),
            Span::styled("    Max Spd:  ", Color::DarkGray),
            Span::raw(format!("{:.1} km/h", d.max_vel)),
        ]),
        Line::from(vec![
            Span::styled("Calories: ", Color::DarkGray),
            Span::raw(format!("{:.0} kcal", d.calories)),
            Span::styled("    TSS: ", Color::DarkGray),
            Span::raw(format!("{:.0}", d.tss)),
        ]),
        Line::from(vec![
            Span::styled("NP:       ", Color::DarkGray),
            Span::raw(format!("{:.0} W", d.normalized_pwr)),
            Span::styled("    IF: ", Color::DarkGray),
            Span::raw(format!("{:.2}", d.ifac)),
        ]),
    ];
    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Left), summary_area);

    let hints = vec![
        Line::from(Span::styled(
            "[S / Y] Save    [D / N] Discard    [R / Esc] Resume",
            Style::default().fg(Color::Yellow),
        ))
        .alignment(Alignment::Center),
        Line::from(Span::styled(
            "Save writes a .fit file + session history (Strava/Garmin ready).",
            Color::DarkGray,
        ))
        .alignment(Alignment::Center),
    ];
    frame.render_widget(Paragraph::new(hints), hint_area);
}

/// Footer rendering function
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

    let pwr_zone = coggan_pwr_model(livedata.crnt_pwr, userdata.profile.ftp);
    let pwr_color = zone2color(pwr_zone);

    let pwrblock = Block::default()
        .title(" POWER ")
        .borders(Borders::ALL)
        .fg(pwr_color)
        .border_type(BorderType::Rounded);

    let pwrinner = pwrblock.inner(pwrrect);
    frame.render_widget(pwrblock, pwrrect);

    let [pwr_header, pwr_main, pwr_graph, pwr_footer] = Layout::vertical([
        Constraint::Length(1), // header
        Constraint::Fill(1),   // big number
        Constraint::Length(3), // graph
        Constraint::Length(2), // footer
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
            format!("FTP {}W", userdata.profile.ftp),
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    frame.render_widget(Paragraph::new(pwr_header_line), pwr_header);

    // --- Big power number ---

    let pwr_pixel = if pwr_main.height >= 6 {
        PixelSize::Full
    } else {
        PixelSize::Quadrant
    };
    let pwr_big = BigText::builder()
        .pixel_size(pwr_pixel)
        .style(Style::default().fg(pwr_color))
        .lines(vec![format!("{}", livedata.crnt_pwr).into()])
        .build();

    frame.render_widget(pwr_big, pwr_main);

    // --- Power graph (braille line over the trailing window) ---

    let pwr_hist: Vec<f64> = app.power_history().iter().map(|&x| x as f64).collect();
    let pwr_hist_max = pwr_hist.iter().copied().fold(0.0, f64::max);
    let pwr_max = (pwr_hist_max).max(userdata.profile.ftp as f64).max(10.0) * 1.1;
    let pwr_points = tail_points(&pwr_hist, pwr_graph.width as usize, |v| v);

    frame.render_widget(line_chart(&pwr_points, pwr_color, pwr_max), pwr_graph);

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
            Span::raw("    "),
            Span::styled("3m ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{}W", livedata.avg_3min_pwr)),
            Span::raw("    "),
            Span::styled("1m ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{}W", livedata.avg_1min_pwr)),
            Span::raw("    "),
            Span::styled("30s ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{}W", livedata.avg_30sec_pwr)),
            Span::raw("    "),
            Span::styled("10s ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{}W", livedata.avg_10sec_pwr)),
            Span::raw("    "),
            Span::styled("3s ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{}W", livedata.avg_3sec_pwr)),
        ]),
    ];

    frame.render_widget(Paragraph::new(pwr_footer_lines), pwr_footer);

    // ============================================================
    // HEART RATE
    // ============================================================

    let hr_zone = olt_hr_model(livedata.crnt_hr, userdata.profile.max_hr);

    let hr_percent = if userdata.profile.max_hr > 0 {
        (livedata.crnt_hr as f32 / userdata.profile.max_hr as f32 * 100.0) as u16
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
        Constraint::Fill(1),   // big number (flexible)
        Constraint::Length(3), // graph (reserved rows, can't overlap the number)
        Constraint::Length(1), // zone/avg footer
    ])
    .areas(hrinner);

    // --- Big heart rate number ---

    let hr_pixel = if hr_top.height >= 6 {
        PixelSize::Full
    } else {
        PixelSize::Quadrant
    };
    let hr_big = BigText::builder()
        .pixel_size(hr_pixel)
        .style(Style::default().fg(Color::Red))
        .lines(vec![format!("{} BPM", livedata.crnt_hr).into()])
        .build();

    frame.render_widget(hr_big, hr_top);

    // --- Heart rate graph (braille line over the trailing window) ---

    let hr_hist: Vec<f64> = app.hr_history().map(|x| x as f64).collect();
    let hr_hist_max = hr_hist.iter().copied().fold(0.0, f64::max);
    let hr_max = hr_hist_max.max(userdata.profile.max_hr as f64).max(1.0) * 1.1;
    let hr_points = tail_points(&hr_hist, hr_graph.width as usize, |v| v);

    frame.render_widget(line_chart(&hr_points, Color::Red, hr_max), hr_graph);

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
            Span::raw("    "),
            Span::styled("TRGT ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{}", livedata.target_hrz)),
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

    let [rpm_value, rpm_graph, rpm_footer] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(2),
        Constraint::Length(1),
    ])
    .areas(rpminner);

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

    let rpm_hist: Vec<f64> = app.rpm_history().map(|x| x as f64).collect();
    let rpm_hist_max = rpm_hist.iter().copied().fold(0.0, f64::max);
    let rpm_max = rpm_hist_max.max(1.0) * 1.1;
    let rpm_points = tail_points(&rpm_hist, rpm_graph.width as usize, |v| v);
    frame.render_widget(
        line_chart(&rpm_points, Color::LightBlue, rpm_max),
        rpm_graph,
    );

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("MAX ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{}", livedata.max_rpm)),
            Span::raw("    "),
            Span::styled("AVG ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{}", livedata.avg_rpm)),
            Span::raw("    "),
            Span::styled("TRGT ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{}", livedata.target_rpm)),
        ]))
        .alignment(Alignment::Center),
        rpm_footer,
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

    let [vel_value, vel_graph, vel_footer] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(2),
        Constraint::Length(1),
    ])
    .areas(velinner);

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

    let vel_hist: Vec<f64> = app.vel_history().iter().map(|&x| x as f64).collect();
    let vel_hist_max = vel_hist.iter().copied().fold(0.0, f64::max);
    let vel_max = vel_hist_max.max(1.0) * 1.1;
    let vel_points = tail_points(&vel_hist, vel_graph.width as usize, |v| v);
    frame.render_widget(line_chart(&vel_points, Color::Green, vel_max), vel_graph);

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("AVG ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{:.1}", livedata.avg_vel)),
            Span::styled("  MAX ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{:.1}", livedata.max_vel)),
        ]))
        .alignment(Alignment::Center),
        vel_footer,
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

    // Distribution of ride time spent in each Coggan zone, shown as a bar
    // scaled to the zone where the rider spent the most time.
    let max_zone = livedata
        .zone_seconds
        .iter()
        .copied()
        .max()
        .unwrap_or(0)
        .max(1);
    let live_zone = coggan_pwr_model(livedata.crnt_pwr, userdata.profile.ftp);

    for (i, rect) in zone_layout.iter().enumerate() {
        let z = (i + 1) as u16;
        let secs = livedata.zone_seconds[i];
        let ratio = secs as f64 / max_zone as f64;
        let is_current = live_zone == z;

        let style = if is_current {
            Style::default()
                .fg(zone2color(z))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(zone2color(z))
        };

        let prefix = if is_current { "▶" } else { " " };
        let label = format!("{prefix}Z{z} {:>2}:{:02}", secs / 60, secs % 60);

        frame.render_widget(
            Gauge::default()
                .ratio(ratio.clamp(0.0, 1.0))
                .label(label)
                .use_unicode(true)
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

    let elapsed = livedata.elapsed_secs;
    let interval_lines: Vec<Line> = match app.workout() {
        Some(w) => {
            let name = w.name.clone().unwrap_or_else(|| "Workout".to_string());
            let total = w.total_seconds;
            match w.step_at(elapsed) {
                Some(step) => {
                    let step_no = w
                        .steps
                        .iter()
                        .position(|s| {
                            s.start_secs == step.start_secs && s.target_power == step.target_power
                        })
                        .unwrap_or(0)
                        + 1;
                    let step_total = w.steps.len();
                    let done = (elapsed.saturating_sub(step.start_secs)) as i64;
                    let remain = (step.end_secs.saturating_sub(elapsed)) as i64;
                    vec![
                        Line::from(Span::styled(
                            format!("{name}  {step_no}/{step_total}"),
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        )),
                        Line::from(""),
                        Line::from(vec![
                            Span::styled("TARGET ", Color::DarkGray),
                            Span::styled(
                                format!("{}W", step.target_power),
                                Style::default()
                                    .fg(Color::Yellow)
                                    .add_modifier(Modifier::BOLD),
                            ),
                        ]),
                        Line::from(""),
                        Line::from(Span::styled(
                            format!("in {:02}:{:02}", done / 60, done % 60),
                            Color::LightGreen,
                        )),
                        Line::from(Span::styled(
                            format!("rem {:02}:{:02}", remain / 60, remain % 60),
                            Color::LightRed,
                        )),
                    ]
                }
                None => {
                    let state = if w.is_finished(elapsed) {
                        "COMPLETE"
                    } else {
                        "PAUSED"
                    };
                    vec![
                        Line::from(Span::styled(
                            format!("{name}  {state}"),
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        )),
                        Line::from(""),
                        Line::from(Span::styled(format!("{elapsed}/{total}s"), Color::DarkGray)),
                    ]
                }
            }
        }
        None => vec![
            Line::from(Span::styled(
                "FREE RIDE",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled("ERC off - ride freely", Color::DarkGray)),
        ],
    };

    frame.render_widget(
        Paragraph::new(interval_lines).alignment(Alignment::Center),
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

    let uptime_h = livedata.elapsed_secs / 3600;
    let uptime_m = (livedata.elapsed_secs / 60) % 60;
    let uptime_s = livedata.elapsed_secs % 60;
    let uptime_str = if uptime_h > 0 {
        format!("{uptime_h:02}:{uptime_m:02}:{uptime_s:02}")
    } else {
        format!("{uptime_m:02}:{uptime_s:02}")
    };

    // Real trainer connection status (name + state, color-coded).
    let (conn_name, conn_state) = app.connection();
    let conn_style = match &conn_state {
        BleUiState::Connected => Color::Green,
        BleUiState::Simulated => Color::Yellow,
        BleUiState::Error(_) => Color::Red,
        _ => Color::DarkGray,
    };

    let systext = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("BT        ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{conn_name}"), Style::default().fg(Color::White)),
            Span::styled(
                format!("  [{}]", conn_state.label()),
                Style::default().fg(conn_style).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("UPTIME    ", Style::default().fg(Color::DarkGray)),
            Span::raw(uptime_str),
        ]),
        Line::from(vec![
            Span::styled("VERSION   ", Style::default().fg(Color::DarkGray)),
            Span::raw(app.version().to_string()),
        ]),
        Line::from(vec![
            Span::styled("FTP       ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{} W", userdata.profile.ftp)),
        ]),
        Line::from(vec![
            Span::styled("MAX HR    ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{} BPM", userdata.profile.max_hr)),
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
    ]);

    frame.render_widget(systext, sysinner);

    // Paused banner — centered over the control panel when ride is paused.
    if app.ride == RideState::Paused {
        let banner_area = centered_rect(42, 14, area);
        frame.render_widget(Clear, banner_area);
        let banner = Paragraph::new(vec![
            Line::from(Span::styled(
                "  PAUSED  ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Center),
            Line::from(Span::styled(
                "Space: resume  •  Q: finish ride",
                Style::default().fg(Color::Gray),
            ))
            .alignment(Alignment::Center),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .alignment(Alignment::Center);
        frame.render_widget(banner, banner_area);
    }
}

// -------------------------------------------------------

fn database_draw(frame: &mut Frame, area: Rect, app: &App) {
    let gray = Style::default().fg(Color::Gray);
    let dark_gray = Style::default().fg(Color::DarkGray);
    let white = Style::default().fg(Color::White);

    let [sidebar_area, list_area] =
        Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)]).areas(area);

    // --- LEFT SIDEBAR: help + summary ---
    let tab_label = match app.database.tab {
        DatabaseTab::Workouts => "Workouts",
        DatabaseTab::Sessions => "Sessions",
    };
    let selected_index = app.database.selected;
    let total = match app.database.tab {
        DatabaseTab::Workouts => app.database.workouts.len(),
        DatabaseTab::Sessions => app.database.sessions.len(),
    };
    let sidebar_lines = vec![
        Line::from(Span::styled(
            " Database ",
            white.add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Active: ", gray),
            Span::styled(tab_label, Style::default().fg(Color::Cyan)),
        ]),
        Line::from(format!(
            "Row {} / {}",
            if total > 0 { selected_index + 1 } else { 0 },
            total
        )),
        Line::from(""),
        Line::from(Span::styled("Keys", white.add_modifier(Modifier::BOLD))),
        Line::from(vec![
            Span::styled("[Up/Down]", Style::default().fg(Color::Yellow)),
            Span::styled(" move row", gray),
        ]),
        Line::from(vec![
            Span::styled("[Left/Right]", Style::default().fg(Color::Yellow)),
            Span::styled(" switch list", gray),
        ]),
        Line::from(vec![
            Span::styled("[Enter]", Style::default().fg(Color::Yellow)),
            Span::styled(" start workout", gray),
        ]),
        Line::from(vec![
            Span::styled("[d]", Style::default().fg(Color::Yellow)),
            Span::styled(" main menu", gray),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(sidebar_lines).block(
            Block::new()
                .borders(Borders::ALL)
                .title(" Info ")
                .fg(Color::DarkGray),
        ),
        sidebar_area,
    );

    // --- RIGHT: ACTIVE LIST ---
    let (title, rows): (&'static str, Vec<Line>) = match app.database.tab {
        DatabaseTab::Workouts => {
            let rows: Vec<Line> = app
                .database
                .workouts
                .iter()
                .enumerate()
                .map(|(i, w)| {
                    let is_sel = i == selected_index;
                    let name = w.path.rsplit('/').next().unwrap_or(&w.path).to_string();
                    let marker = if is_sel { " ▶ " } else { "   " };
                    Line::from(vec![
                        Span::styled(
                            marker,
                            if is_sel {
                                Style::default()
                                    .fg(Color::Cyan)
                                    .add_modifier(Modifier::BOLD)
                            } else {
                                gray
                            },
                        ),
                        Span::styled(
                            name.clone(),
                            if is_sel {
                                white.add_modifier(Modifier::BOLD)
                            } else {
                                gray
                            },
                        ),
                    ])
                })
                .collect();
            (" Workouts ", rows)
        }
        DatabaseTab::Sessions => {
            let header = Line::from(vec![Span::styled(
                "  Date                        Dist   AvgP   MaxP",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]);
            let mut rows = vec![header];
            for (i, s) in app.database.sessions.iter().enumerate() {
                let is_sel = i == selected_index;
                let line = Line::from(vec![
                    Span::styled(
                        if is_sel { "▶ " } else { "  " },
                        if is_sel {
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            gray
                        },
                    ),
                    Span::styled(
                        format!(
                            "{:>24} {:>7.1} {:>6} {:>6}",
                            s.recorded_at, s.total_distance, s.avg_power, s.max_power
                        ),
                        if is_sel { white } else { gray },
                    ),
                ]);
                rows.push(line);
            }
            if rows.len() == 1 {
                rows.push(Line::from(Span::styled(
                    "  No sessions recorded yet.",
                    dark_gray,
                )));
            }
            (" Session History ", rows)
        }
    };

    frame.render_widget(
        Paragraph::new(rows).block(
            Block::new()
                .borders(Borders::ALL)
                .title(title)
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
        SettingsSelection::Bluetooth => {
            let (name, state) = app.connection();
            let state_label = state.label().to_string();
            let state_color = match &state {
                BleUiState::Connected => Color::Green,
                BleUiState::Simulated => Color::Yellow,
                BleUiState::Error(_) => Color::Red,
                _ => Color::DarkGray,
            };
            let detail = match &state {
                BleUiState::Error(e) => format!("{e}"),
                _ => String::new(),
            };
            let mut lines = vec![
                Line::from(Span::styled(
                    "Bluetooth Devices",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled("-----------------", Color::DarkGray)),
                Line::from(vec![
                    Span::styled("Trainer: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(name, Style::default().fg(Color::White)),
                ]),
                Line::from(vec![
                    Span::styled("Status:  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(state_label, Style::default().fg(state_color)),
                ]),
            ];
            if !detail.is_empty() {
                lines.push(Line::from(Span::styled(detail, Color::Red)));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "The Tacx Flux S2 connects via FTMS (Fitness Machine Service).",
                Color::DarkGray,
            )));
            Paragraph::new(lines)
        }
        SettingsSelection::System => Paragraph::new(format!(
            "System Information\n------------------\nVersion: {}",
            app.version()
        )),
        SettingsSelection::User => {
            let profile = &app.userdata().profile;
            let st = &app.settings;
            let field_line = |label: &str, value: String, f: SettingsField| {
                let active = st.field == f;
                let display = if active && st.editing {
                    format!("> {}: {}_", label, st.draft)
                } else if active {
                    format!("> {}: {}", label, value)
                } else {
                    format!("  {}: {}", label, value)
                };
                Line::from(vec![Span::styled(
                    display,
                    if active {
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Gray)
                    },
                )])
            };
            let mut lines = vec![
                Line::from("User Settings"),
                Line::from("--------------"),
                Line::from(Span::styled(
                    "Edit rider profile. Changes apply live.",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(""),
            ];
            lines.push(field_line(
                "Name",
                profile.username.clone(),
                SettingsField::Name,
            ));
            lines.push(field_line(
                "Weight (kg)",
                format!("{:.1}", profile.weight),
                SettingsField::Weight,
            ));
            lines.push(field_line(
                "Height (cm)",
                format!("{:.1}", profile.height),
                SettingsField::Height,
            ));
            lines.push(field_line(
                "FTP (W)",
                format!("{}", profile.ftp),
                SettingsField::Ftp,
            ));
            lines.push(field_line(
                "Max HR (BPM)",
                format!("{}", profile.max_hr),
                SettingsField::MaxHr,
            ));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                if st.editing {
                    "Editing: Enter to save, Esc to cancel, Backspace to delete"
                } else {
                    "Up/Down: select field   Enter: edit   Tab: switch panel"
                },
                Style::default().fg(Color::DarkGray),
            )));
            Paragraph::new(lines)
        }
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

    // Overlays (rendered on top of everything).
    let full = frame.area();
    if app.ride == RideState::Summary {
        render_summary(frame, app, full);
    } else if app.confirm_quit {
        render_confirm_quit(frame, full);
    } else if app.is_loading() {
        render_loading(frame, full);
    }
}
