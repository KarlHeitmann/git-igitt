use crate::app::{ActiveView, App, DiffMode};
use crate::dialogs::FileDialog;
use crate::widgets::branches_view::{BranchList, BranchListItem};
use crate::widgets::commit_view::CommitView;
use crate::widgets::files_view::{FileList, FileListItem};
use crate::widgets::graph_view::GraphView;
use crate::widgets::models_view::ModelListState;
use tui::backend::Backend;
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Text};
use tui::widgets::{
    Block, BorderType, Borders, Clear, List, ListItem as TuiListItem, Paragraph, Wrap,
};
use tui::Frame;

pub fn draw_open_repo<B: Backend>(f: &mut Frame<B>, dialog: &mut FileDialog) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Min(0)].as_ref())
        .split(f.size());

    let top_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)].as_ref())
        .split(chunks[0]);

    let location_block = Block::default().borders(Borders::ALL).title(" Path ");

    let paragraph = Paragraph::new(format!("{}", &dialog.location.display())).block(location_block);
    f.render_widget(paragraph, top_chunks[0]);

    let help = Paragraph::new("  Navigate with Arrows, confirm with Enter.");
    f.render_widget(help, top_chunks[1]);

    let list_block = Block::default()
        .borders(Borders::ALL)
        .title(" Open repository ");

    let items: Vec<_> = dialog
        .dirs
        .iter()
        .map(|f| TuiListItem::new(&f[..]))
        .collect();

    let mut list = List::new(items).block(list_block).highlight_symbol("> ");

    if dialog.color {
        list = list.highlight_style(Style::default().add_modifier(Modifier::UNDERLINED));
    }

    f.render_stateful_widget(list, chunks[1], &mut dialog.state);

    if let Some(error) = &dialog.error_message {
        draw_error_dialog(f, f.size(), error, dialog.color);
    }
}
pub fn draw<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    if let ActiveView::Help(scroll) = app.active_view {
        draw_help(f, f.size(), scroll);
        return;
    }

    if let (ActiveView::Models, Some(model_state)) = (&app.active_view, &mut app.models_state) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)].as_ref())
            .split(f.size());

        let help = Paragraph::new("  Enter = confirm, P = permanent, Esc = abort.");
        f.render_widget(help, chunks[0]);

        draw_models(f, chunks[1], app.color, model_state);
        return;
    }

    if app.is_fullscreen {
        match app.active_view {
            ActiveView::Branches => draw_branches(f, f.size(), app),
            ActiveView::Graph => draw_graph(f, f.size(), app),
            ActiveView::Commit => draw_commit(f, f.size(), app),
            ActiveView::Files => draw_files(f, f.size(), app),
            ActiveView::Diff => draw_diff(f, f.size(), app),
            _ => {}
        }
    } else {
        let base_split = if app.horizontal_split {
            Direction::Horizontal
        } else {
            Direction::Vertical
        };
        let sub_split = if app.horizontal_split {
            Direction::Vertical
        } else {
            Direction::Horizontal
        };

        let show_branches = app.show_branches || app.active_view == ActiveView::Branches;

        let top_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Length(if show_branches { 25 } else { 0 }),
                    Constraint::Min(0),
                ]
                .as_ref(),
            )
            .split(f.size());

        let chunks = Layout::default()
            .direction(base_split)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(top_chunks[1]);

        let right_chunks = Layout::default()
            .direction(sub_split)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(chunks[1]);

        match app.active_view {
            ActiveView::Files | ActiveView::Diff => draw_diff(f, chunks[0], app),
            _ => draw_graph(f, chunks[0], app),
        }
        if show_branches {
            draw_branches(f, top_chunks[0], app);
        }
        draw_commit(f, right_chunks[0], app);
        draw_files(f, right_chunks[1], app);
    }

    if let Some(error) = &app.error_message {
        draw_error_dialog(f, f.size(), error, app.color);
    }
}

fn draw_graph<B: Backend>(f: &mut Frame<B>, target: Rect, app: &mut App) {
    let mut block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Graph - {} ", app.repo_name));
    if app.active_view == ActiveView::Graph {
        block = block.border_type(BorderType::Thick);
    }

    let mut graph = GraphView::default().block(block).highlight_symbol(">", "#");

    if app.color {
        graph = graph.highlight_style(Style::default().add_modifier(Modifier::UNDERLINED));
    }

    f.render_stateful_widget(graph, target, &mut app.graph_state);
}

fn draw_branches<B: Backend>(f: &mut Frame<B>, target: Rect, app: &mut App) {
    let color = app.color;
    if let Some(state) = &mut app.graph_state.branches {
        let mut block = Block::default().borders(Borders::ALL).title(" Branches ");
        if app.active_view == ActiveView::Branches {
            block = block.border_type(BorderType::Thick);
        }

        let items: Vec<_> = state
            .items
            .iter()
            .map(|item| {
                BranchListItem::new(
                    if color {
                        Span::styled(&item.name, Style::default().fg(Color::Indexed(item.color)))
                    } else {
                        Span::raw(&item.name)
                    },
                    &item.branch_type,
                )
            })
            .collect();

        let mut list = BranchList::new(items).block(block).highlight_symbol("> ");

        if color {
            list = list.highlight_style(Style::default().add_modifier(Modifier::UNDERLINED));
        }

        f.render_stateful_widget(list, target, &mut state.state);
    } else {
        let mut block = Block::default().borders(Borders::ALL).title("Files");
        if app.active_view == ActiveView::Files {
            block = block.border_type(BorderType::Thick);
        }
        f.render_widget(block, target);
    }
}

fn draw_commit<B: Backend>(f: &mut Frame<B>, target: Rect, app: &mut App) {
    let mut block = Block::default().borders(Borders::ALL).title(" Commit ");
    if app.active_view == ActiveView::Commit {
        block = block.border_type(BorderType::Thick);
    }

    let commit = CommitView::default().block(block).highlight_symbol(">");

    f.render_stateful_widget(commit, target, &mut app.commit_state);
}

fn draw_files<B: Backend>(f: &mut Frame<B>, target: Rect, app: &mut App) {
    let color = app.color;
    if let Some(state) = &mut app.commit_state.content {
        let mut block = Block::default().borders(Borders::ALL).title(format!(
            " Files ({}..{}) ",
            &state.compare_oid.to_string()[..7],
            &state.oid.to_string()[..7]
        ));
        if app.active_view == ActiveView::Files {
            block = block.border_type(BorderType::Thick);
        }

        let items: Vec<_> = state
            .diffs
            .items
            .iter()
            .map(|item| {
                if color {
                    let style = Style::default().fg(item.diff_type.to_color());
                    FileListItem::new(
                        Span::styled(&item.file, style),
                        Span::styled(format!("{} ", item.diff_type.to_string()), style),
                    )
                } else {
                    FileListItem::new(
                        Span::raw(&item.file),
                        Span::raw(format!("{} ", item.diff_type.to_string())),
                    )
                }
            })
            .collect();

        let mut list = FileList::new(items).block(block).highlight_symbol("> ");

        if color {
            list = list.highlight_style(Style::default().add_modifier(Modifier::UNDERLINED));
        }

        f.render_stateful_widget(list, target, &mut state.diffs.state);
    } else {
        let mut block = Block::default().borders(Borders::ALL).title("Files");
        if app.active_view == ActiveView::Files {
            block = block.border_type(BorderType::Thick);
        }
        f.render_widget(block, target);
    }
}

fn draw_diff<B: Backend>(f: &mut Frame<B>, target: Rect, app: &mut App) {
    if let Some(state) = &app.diff_state.content {
        let title = match app.diff_options.diff_mode {
            DiffMode::Diff => format!(
                " Diff ({}..{}) ",
                &state.compare_oid.to_string()[..7],
                &state.oid.to_string()[..7]
            ),
            DiffMode::Old => format!(" Diff (old: {}) ", &state.compare_oid.to_string()[..7],),
            DiffMode::New => format!(" Diff (new: {}) ", &state.oid.to_string()[..7],),
        };
        let mut block = Block::default().borders(Borders::ALL).title(title);
        if app.active_view == ActiveView::Diff {
            block = block.border_type(BorderType::Thick);
        }

        let styles = [
            Style::default().fg(Color::LightGreen),
            Style::default().fg(Color::LightRed),
            Style::default().fg(Color::LightBlue),
            Style::default(),
        ];

        let (space_old_ln, space_new_ln, empty_old_ln, empty_new_ln) = if app.line_numbers {
            let mut max_old_ln = None;
            let mut max_new_ln = None;

            for (_, old_ln, new_ln) in state.diffs.iter().rev() {
                if max_old_ln.is_none() {
                    if let Some(old_ln) = old_ln {
                        max_old_ln = Some(*old_ln);
                    }
                }
                if max_new_ln.is_none() {
                    if let Some(new_ln) = new_ln {
                        max_new_ln = Some(*new_ln);
                    }
                }
                if max_old_ln.is_some() && max_new_ln.is_some() {
                    break;
                }
            }

            let space_old_ln =
                std::cmp::max(3, (max_old_ln.unwrap_or(0) as f32).log10().floor() as usize);
            let space_new_ln =
                std::cmp::max(3, (max_new_ln.unwrap_or(0) as f32).log10().floor() as usize) + 1;

            (
                space_old_ln,
                space_new_ln,
                " ".repeat(space_old_ln),
                " ".repeat(space_new_ln),
            )
        } else {
            (0, 0, String::new(), String::new())
        };

        let mut text = Text::from("");
        for (line, old_ln, new_ln) in &state.diffs {
            let ln = if line.starts_with("@@ ") {
                if let Some(pos) = line.find(" @@ ") {
                    &line[..pos + 3]
                } else {
                    line
                }
            } else {
                line
            };

            if app.line_numbers && (old_ln.is_some() || new_ln.is_some()) {
                let l1 = old_ln
                    .map(|v| format!("{:>width$}", v, width = space_old_ln))
                    .unwrap_or_else(|| empty_old_ln.clone());
                let l2 = new_ln
                    .map(|v| format!("{:>width$}", v, width = space_new_ln))
                    .unwrap_or_else(|| empty_new_ln.clone());
                let fmt = format!("{}{}|", l1, l2);

                text.extend(style_diff_line(Some(fmt), ln, &styles, app.color));
            } else {
                text.extend(style_diff_line(None, ln, &styles, app.color));
            }
        }

        let paragraph = Paragraph::new(text).block(block).scroll(state.scroll);

        f.render_widget(paragraph, target);
    } else {
        let mut block = Block::default().borders(Borders::ALL).title(" Diff ");
        if app.active_view == ActiveView::Diff {
            block = block.border_type(BorderType::Thick);
        }
        f.render_widget(block, target);
    }
}

fn style_diff_line<'a>(
    prefix: Option<String>,
    line: &'a str,
    styles: &'a [Style; 4],
    color: bool,
) -> Text<'a> {
    if !color {
        if let Some(prefix) = prefix {
            Text::raw(format!("{}{}", prefix, line))
        } else {
            Text::raw(line)
        }
    } else {
        let style = if line.starts_with('+') {
            styles[0]
        } else if line.starts_with('-') {
            styles[1]
        } else if line.starts_with('@') {
            styles[2]
        } else {
            styles[3]
        };
        if let Some(prefix) = prefix {
            Text::styled(format!("{}{}", prefix, line), style)
        } else {
            Text::styled(line, style)
        }
    }
}

fn draw_models<B: Backend>(
    f: &mut Frame<B>,
    target: Rect,
    color: bool,
    state: &mut ModelListState,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Branching model ");

    let items: Vec<_> = state
        .models
        .iter()
        .map(|m| TuiListItem::new(&m[..]))
        .collect();

    let mut list = List::new(items).block(block).highlight_symbol("> ");

    if color {
        list = list.highlight_style(Style::default().add_modifier(Modifier::UNDERLINED));
    }

    f.render_stateful_widget(list, target, &mut state.state);
}

fn draw_help<B: Backend>(f: &mut Frame<B>, target: Rect, scroll: u16) {
    let block = Block::default().borders(Borders::ALL).title(" Help ");

    let paragraph = Paragraph::new(
        "F1/H               Show this help\n\
         Q                  Quit\n\
         Ctrl + O           Open repository\n\
         M                  Set branching model\n\
         \n\
         Up/Down            Select / navigate / scroll\n\
         Shift + Up/Down    Navigate fast\n\
         Home/End           Navigate to first/last\n\
         Ctrl + Up/Down     Secondary selection (compare arbitrary commits)\n\
         Backspace          Clear secondary selection\n\
         Ctrl + Left/Right  Scroll horizontal\n\
         Enter              Jump to selected branch/tag\n\
         \n\
         +/-                Increase/decrease number of diff context lines
         \n\
         Left/Right         Change panel\n\
         Tab                Panel to fullscreen\n\
         Ecs                Return to default view\n\
         L                  Toggle horizontal/vertical layout\n\
         B                  Toggle show branch list\n\
         Ctrl + L           Toggle line numbers in diff\n\
         \n\
         R                  Reload repository graph",
    )
    .block(block)
    .scroll((scroll, 0));

    f.render_widget(paragraph, target);
}

fn draw_error_dialog<B: Backend>(f: &mut Frame<B>, target: Rect, error: &str, color: bool) {
    let mut block = Block::default()
        .title(" Error - Press Enter to continue ")
        .borders(Borders::ALL)
        .border_type(BorderType::Thick);

    if color {
        block = block.border_style(Style::default().fg(Color::LightRed));
    }

    let paragraph = Paragraph::new(error).block(block).wrap(Wrap { trim: true });

    let area = centered_rect(60, 12, target);
    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}
/// helper function to create a centered rect using up
/// certain percentage of the available rect `r`
fn centered_rect(size_x: u16, size_y: u16, r: Rect) -> Rect {
    let size_x = std::cmp::min(size_x, r.width);
    let size_y = std::cmp::min(size_y, r.height);

    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length((r.height - size_y) / 2),
                Constraint::Min(size_y),
                Constraint::Length((r.height - size_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Length((r.width - size_x) / 2),
                Constraint::Min(size_x),
                Constraint::Length((r.width - size_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}
