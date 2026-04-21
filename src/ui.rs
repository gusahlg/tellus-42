use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use crate::app::{App, COMMAND_HEIGHT, Mode, layer_name};

pub fn draw(frame: &mut Frame<'_>, app: &mut App) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(COMMAND_HEIGHT)])
        .split(frame.area());

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(app.sidebar_width()), Constraint::Min(1)])
        .split(root[0]);

    draw_sidebar(frame, app, body[0]);
    draw_viewport(frame, app, body[1]);
    draw_command_bar(frame, app, root[1]);
}

fn draw_sidebar(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let theme = app.theme();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.panel_border))
        .style(Style::default().bg(theme.sidebar_bg));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(10),
            Constraint::Length(7),
            Constraint::Min(8),
            Constraint::Length(5),
        ])
        .split(inner);

    draw_title(frame, sections[0]);
    draw_overview(frame, app, sections[1]);
    draw_layer_panel(frame, app, sections[2]);
    draw_mapping_panel(frame, app, sections[3]);
    draw_status_panel(frame, app, sections[4]);
}

fn draw_viewport(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let theme = app.theme();
    let block = Block::default()
        .title(format!("Canvas [{}]", layer_name(app.active_layer())))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.panel_border));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(
        Paragraph::new("")
            .style(Style::default().bg(theme.grid_bg)),
        inner,
    );

    let (tile_w, tile_h) = app.tile_size();
    let tiles_across = (inner.width / tile_w.max(1)).max(1);
    let tiles_down = (inner.height / tile_h.max(1)).max(1);
    app.ensure_cursor_visible((tiles_across, tiles_down));

    for dy in 0..tiles_down {
        for dx in 0..tiles_across {
            let x = app.view_origin().0.saturating_add(dx);
            let y = app.view_origin().1.saturating_add(dy);
            if x >= app.level().width || y >= app.level().height {
                continue;
            }

            let cell_area = Rect {
                x: inner.x + dx * tile_w,
                y: inner.y + dy * tile_h,
                width: tile_w,
                height: tile_h,
            };
            draw_tile(frame, app, x, y, cell_area);
        }
    }
}

fn draw_tile(frame: &mut Frame<'_>, app: &App, x: u16, y: u16, area: Rect) {
    let theme = app.theme();
    let tile_id = app.visible_tile_id(x, y).unwrap_or(0);
    let is_cursor = (x, y) == app.cursor();
    let texture = app.tile_texture(app.active_layer(), tile_id);
    let (gap_x, gap_y) = app.tile_gap();

    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new("").style(Style::default().bg(theme.grid_bg)),
        area,
    );

    let tile_area = inset_rect_end(area, gap_x, gap_y);
    if tile_area.width == 0 || tile_area.height == 0 {
        return;
    }

    frame.render_widget(
        Paragraph::new("").style(Style::default().bg(theme.tile_bg)),
        tile_area,
    );

    let content_area = if is_cursor && tile_area.width > 2 && tile_area.height > 2 {
        frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .border_style(
                    Style::default()
                        .fg(cursor_color(app.mode(), app))
                        .bg(theme.tile_bg)
                        .add_modifier(Modifier::BOLD),
                )
                .style(Style::default().bg(theme.tile_bg)),
            tile_area,
        );
        inset_rect_uniform(tile_area, 1)
    } else {
        tile_area
    };

    if content_area.width == 0 || content_area.height == 0 {
        return;
    }

    let cell_rows = content_area.height.max(1);
    let colors = app.texture_colors(texture, content_area.width.max(1), cell_rows);

    for (row_index, row) in colors.iter().enumerate().take(content_area.height as usize) {
        let mut spans = Vec::with_capacity(row.len());
        for &(top, bottom) in row {
            let style = Style::default()
                .fg(top)
                .bg(if matches!(bottom, Color::Reset) {
                    theme.tile_bg
                } else {
                    bottom
                });
            spans.push(Span::styled("▀", style));
        }

        if spans.is_empty() {
            spans.push(Span::styled(
                " ".repeat(content_area.width as usize),
                Style::default().bg(theme.tile_bg),
            ));
        }

        let line = Line::from(spans);
        frame.render_widget(
            Paragraph::new(line),
            Rect {
                x: content_area.x,
                y: content_area.y + row_index as u16,
                width: content_area.width,
                height: 1,
            },
        );
    }

    if texture.is_none() {
        let label = tile_id.to_string();
        frame.render_widget(
            Paragraph::new(Line::from(vec![Span::styled(
                center_text(&label, content_area.width as usize),
                Style::default().fg(Color::White).bg(theme.tile_bg),
            )])),
            Rect {
                x: content_area.x,
                y: content_area.y + content_area.height.saturating_sub(1) / 2,
                width: content_area.width,
                height: 1,
            },
        );
    }
}

fn draw_command_bar(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let theme = app.theme();
    let title = match app.mode() {
        Mode::Command => "Command",
        Mode::Insert => "Insert",
        Mode::Normal => "Status",
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.panel_border));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let content = match app.mode() {
        Mode::Command => vec![Line::from(Span::styled(
            format!(":{}", app.command_buffer()),
            Style::default().fg(theme.accent_text),
        ))],
        Mode::Insert => vec![Line::from(Span::styled(
            "Insert mode: move with h j k l / arrows, press 1-9 to paint, Esc to leave",
            Style::default().fg(theme.warning_text),
        ))],
        Mode::Normal => vec![Line::from(Span::styled(
            "Normal mode: i insert, J/K layer, +/- zoom, : commands, q via :q",
            Style::default().fg(theme.muted_text),
        ))],
    };

    frame.render_widget(Paragraph::new(content), inner);
}

fn draw_title(frame: &mut Frame<'_>, area: Rect) {
    let title = vec![
        Line::from(Span::styled(
            "╭─ Tellus 42 ─╮",
            Style::default()
                .fg(Color::Rgb(188, 198, 214))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "│ level editor │",
            Style::default().fg(Color::Rgb(128, 136, 148)),
        )),
    ];
    frame.render_widget(Paragraph::new(title).wrap(Wrap { trim: false }), area);
}

fn draw_overview(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let theme = app.theme();
    let file = app
        .path()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<unsaved>".to_string());
    let lines = vec![
        section_header("STATE", theme),
        kv_line("file", &file, theme.accent_text, theme),
        kv_line("size", &format!("{}x{}", app.level().width, app.level().height), theme.panel_text, theme),
        kv_line("dirty", if app.dirty() { "yes" } else { "no" }, if app.dirty() { theme.warning_text } else { theme.success_text }, theme),
        kv_line("mode", &format!("{:?}", app.mode()), theme.panel_text, theme),
        kv_line("zoom", &app.zoom().to_string(), theme.panel_text, theme),
        kv_line("cursor", &format!("{}, {}", app.cursor().0, app.cursor().1), theme.panel_text, theme),
        kv_line("view", &format!("{}, {}", app.view_origin().0, app.view_origin().1), theme.panel_text, theme),
    ];
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}

fn draw_layer_panel(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let theme = app.theme();
    let mut lines = vec![section_header("LAYERS", theme), spacer()];
    for layer in [app.active_layer(), next_layer(app.active_layer()), prev_layer(app.active_layer())] {
        let selected = layer == app.active_layer();
        let marker = if selected { "▶" } else { "•" };
        let style = if selected {
            Style::default().fg(theme.accent_text).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.muted_text)
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{marker} "), style),
            Span::styled(layer_name(layer), style),
            Span::styled(
                format!("  [{}]", app.layer_assets(layer).tiles.len()),
                Style::default().fg(theme.muted_text),
            ),
        ]));
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}

fn draw_mapping_panel(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let theme = app.theme();
    let folder = app
        .layer_assets(app.active_layer())
        .folder
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<none>".to_string());

    let mut lines = vec![
        section_header("MAPPING", theme),
        spacer(),
        kv_line("folder", &folder, theme.panel_text, theme),
        spacer(),
    ];

    if app.layer_assets(app.active_layer()).tiles.is_empty() {
        lines.push(Line::from(Span::styled(
            "No mapped textures yet.",
            Style::default().fg(theme.muted_text),
        )));
        lines.push(Line::from(Span::styled(
            "Use :map <layer> <folder>",
            Style::default().fg(theme.muted_text),
        )));
    } else {
        lines.push(section_subheader("TILES", theme));
        for tile in app.layer_assets(app.active_layer()).tiles.iter().take(9) {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{:>2}", tile.id),
                    Style::default().fg(theme.success_text).add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled(&tile.name, Style::default().fg(theme.panel_text)),
            ]));
        }
    }

    lines.push(spacer());
    lines.push(section_subheader("KEYS", theme));
    lines.push(help_line("move", "h j k l / arrows", theme));
    lines.push(help_line("layer", "J / K", theme));
    lines.push(help_line("zoom", "+ / -", theme));
    lines.push(help_line("paint", "i then 0-9", theme));
    lines.push(help_line("cmd", ":", theme));

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}

fn draw_status_panel(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let theme = app.theme();
    let is_error = app.status().starts_with("Error:");
    let lines = vec![
        section_header("STATUS", theme),
        spacer(),
        Line::from(Span::styled(
            app.status(),
            Style::default().fg(if is_error { theme.error_text } else { theme.warning_text }),
        )),
    ];
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}

fn next_layer(layer: tellus_level::LayerKind) -> tellus_level::LayerKind {
    match layer {
        tellus_level::LayerKind::Ground => tellus_level::LayerKind::Detail,
        tellus_level::LayerKind::Detail => tellus_level::LayerKind::Logic,
        tellus_level::LayerKind::Logic => tellus_level::LayerKind::Ground,
    }
}

fn prev_layer(layer: tellus_level::LayerKind) -> tellus_level::LayerKind {
    match layer {
        tellus_level::LayerKind::Ground => tellus_level::LayerKind::Logic,
        tellus_level::LayerKind::Detail => tellus_level::LayerKind::Ground,
        tellus_level::LayerKind::Logic => tellus_level::LayerKind::Detail,
    }
}

fn inset_rect_end(area: Rect, gap_x: u16, gap_y: u16) -> Rect {
    Rect {
        x: area.x,
        y: area.y,
        width: area.width.saturating_sub(gap_x),
        height: area.height.saturating_sub(gap_y),
    }
}

fn inset_rect_uniform(area: Rect, margin: u16) -> Rect {
    let horizontal = margin.saturating_mul(2);
    let vertical = margin.saturating_mul(2);
    Rect {
        x: area.x.saturating_add(margin),
        y: area.y.saturating_add(margin),
        width: area.width.saturating_sub(horizontal),
        height: area.height.saturating_sub(vertical),
    }
}

fn cursor_color(mode: Mode, app: &App) -> Color {
    match mode {
        Mode::Insert => app.theme().cursor_insert,
        Mode::Command => app.theme().cursor_command,
        Mode::Normal => app.theme().cursor_normal,
    }
}

fn section_header(label: &str, theme: &crate::config::UiTheme) -> Line<'static> {
    Line::from(Span::styled(
        label.to_string(),
        Style::default()
            .fg(theme.accent_text)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
    ))
}

fn section_subheader(label: &str, theme: &crate::config::UiTheme) -> Line<'static> {
    Line::from(Span::styled(
        label.to_string(),
        Style::default().fg(theme.muted_text).add_modifier(Modifier::BOLD),
    ))
}

fn spacer() -> Line<'static> {
    Line::from("")
}

fn kv_line(label: &str, value: &str, value_color: Color, theme: &crate::config::UiTheme) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label:<6}"), Style::default().fg(theme.muted_text)),
        Span::raw(" "),
        Span::styled(value.to_string(), Style::default().fg(value_color)),
    ])
}

fn help_line(label: &str, value: &str, theme: &crate::config::UiTheme) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label:<5}"), Style::default().fg(theme.muted_text)),
        Span::raw(" "),
        Span::styled(value.to_string(), Style::default().fg(theme.panel_text)),
    ])
}

fn center_text(text: &str, width: usize) -> String {
    if width <= text.len() {
        return text.chars().take(width).collect();
    }

    let padding = width - text.len();
    let left = padding / 2;
    let right = padding - left;
    format!("{}{}{}", " ".repeat(left), text, " ".repeat(right))
}
