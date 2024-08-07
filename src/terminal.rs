use crossterm::{
	event::{
		self, Event,
		KeyCode::{self, Char},
		KeyEvent, KeyModifiers,
	},
	execute,
	terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use git2::{Oid, Repository};
use std::{
	error::Error,
	io::{self, Stdout},
	path::{Path, PathBuf},
};
use tui::{
	backend::CrosstermBackend,
	layout::{Constraint, Direction, Layout, Rect},
	style::{Color, Modifier, Style},
	text::{Line, Span, Text},
	widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
	Frame, Terminal,
};

use crate::git;

pub struct App<'a> {
	pub blame: Vec<git::BlameHunk<'a>>,
	blame_state: ListState,
	repo: &'a Repository,
	commit_stack: Vec<CommitPath>,      // pushed by `b`, popped by `B`
	right_panel: Option<Text<'static>>, // activated by `w` or <enter>
	line_history_scroll: u16,
	popup: Option<Text<'static>>,
	search: Option<Search>,
	line_number: Option<String>,
}

struct Search {
	editing: bool,
	query: String,
}

struct CommitPath {
	commit: Oid,
	path: PathBuf,
}

impl App<'_> {
	pub fn new<'a>(repo: &'a Repository, rel_path: &'a Path, commit: Oid) -> App<'a> {
		App {
			blame: vec![],
			blame_state: ListState::default(),
			repo,
			commit_stack: vec![CommitPath {
				commit,
				path: rel_path.to_owned(),
			}],
			right_panel: None,
			line_history_scroll: 0,
			popup: None,
			search: None,
			line_number: None,
		}
	}
}

type CrosstermTerm = Terminal<CrosstermBackend<Stdout>>;

pub fn setup() -> Result<CrosstermTerm, Box<dyn Error>> {
	enable_raw_mode()?;
	let mut stdout = io::stdout();
	execute!(stdout, EnterAlternateScreen)?;
	let backend = CrosstermBackend::new(stdout);
	Ok(Terminal::new(backend)?)
}

pub fn teardown(terminal: &mut CrosstermTerm) {
	_ = disable_raw_mode();
	_ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
	_ = terminal.show_cursor();
}

pub fn run_app(terminal: &mut CrosstermTerm, mut app: App) -> Result<(), Box<dyn Error>> {
	loop {
		terminal.draw(|frame| ui(frame, &mut app))?;
		if let Event::Key(key) = event::read()? {
			match handle_input(&key, &mut app, &terminal.size()?) {
				Ok(false) => {
					return Ok(());
				}
				Ok(true) => {} // ignored
				Err(err) => app.popup = Some(format!("{}", err).into()),
			}
		}
	}
}

// returns whether to continue running the app
fn handle_input(key: &KeyEvent, app: &mut App, term_size: &Rect) -> Result<bool, Box<dyn Error>> {
	if app.popup.is_some() {
		// clear the popup on any key press
		app.popup = None;
		return Ok(true);
	}

	if let Some(search) = &mut app.search {
		if search.editing {
			match key {
				KeyEvent { code: KeyCode::Esc, .. }
				| KeyEvent {
					code: Char('c'),
					modifiers: KeyModifiers::CONTROL,
					..
				} => {
					app.search = None;
				}
				KeyEvent {
					code: Char('u'),
					modifiers: KeyModifiers::CONTROL,
					..
				} => {
					search.query.clear();
				}
				KeyEvent { code: Char(c), .. } => {
					search.query.push(*c);
				}
				KeyEvent {
					code: KeyCode::Backspace,
					..
				} => {
					search.query.pop();
				}
				KeyEvent {
					code: KeyCode::Enter, ..
				} => {
					search.editing = false;
					handle_search(&app.blame, &search.query, &mut app.blame_state, true);
				}
				_ => {} // ignored
			}
			return Ok(true);
		}
	} else if let Some(line_number) = &mut app.line_number {
		match key {
			KeyEvent { code: KeyCode::Esc, .. }
			| KeyEvent {
				code: Char('c'),
				modifiers: KeyModifiers::CONTROL,
				..
			} => app.line_number = None,
			KeyEvent {
				code: Char('u'),
				modifiers: KeyModifiers::CONTROL,
				..
			} => {
				line_number.clear();
			}
			KeyEvent { code: Char(c), .. } => {
				if '0' <= *c && *c <= '9' {
					line_number.push(*c);
				}
			}
			KeyEvent {
				code: KeyCode::Backspace,
				..
			} => {
				line_number.pop();
			}
			KeyEvent {
				code: KeyCode::Enter, ..
			} => {
				if let Ok(index) = line_number.parse::<usize>() {
					app.blame_state.select(Some(index.clamp(1, app.blame.len()) - 1));
					app.line_number = None;
				}
			}
			_ => {} // ignored
		}
		return Ok(true);
	}

	match key {
		// scroll
		KeyEvent {
			code: Char('j') | KeyCode::Down,
			..
		} => scroll(app, term_size, 1),
		KeyEvent {
			code: Char('k') | KeyCode::Up,
			..
		} => scroll(app, term_size, -1),
		KeyEvent { code: Char('d'), .. }
		| KeyEvent {
			code: KeyCode::PageDown,
			..
		} => scroll(app, term_size, (term_size.height / 2).try_into().unwrap()),
		KeyEvent { code: Char('u'), .. }
		| KeyEvent {
			code: KeyCode::PageUp, ..
		} => scroll(app, term_size, -i16::try_from(term_size.height / 2).unwrap()),
		KeyEvent { code: Char('g'), .. }
		| KeyEvent {
			code: KeyCode::Home, ..
		} => match &app.right_panel {
			Some(_) => app.line_history_scroll = 0,
			None => app.blame_state.select(Some(0)),
		},
		KeyEvent { code: Char('G'), .. } | KeyEvent { code: KeyCode::End, .. } => match &app.right_panel {
			Some(line_history) => {
				app.line_history_scroll = u16::try_from(line_history.height())
					.unwrap()
					.saturating_sub(term_size.height)
			}
			None => app.blame_state.select(Some(app.blame.len() - 1)),
		},
		KeyEvent { code: Char(':'), .. } => {
			app.line_number = Some(String::new());
		}
		// search
		KeyEvent { code: Char('/'), .. } => {
			app.search = Some(Search {
				editing: true,
				query: String::new(),
			});
		}
		KeyEvent { code: Char('n'), .. } => {
			if let Some(search) = &app.search {
				handle_search(&app.blame, &search.query, &mut app.blame_state, true);
			}
		}
		KeyEvent { code: Char('N'), .. } => {
			if let Some(search) = &app.search {
				handle_search(&app.blame, &search.query, &mut app.blame_state, false);
			}
		}
		// other interactions
		KeyEvent {
			code: KeyCode::Enter, ..
		} => {
			if let Some(index) = app.blame_state.selected() {
				app.right_panel = Some(git::show(app.repo, app.blame[index].commit));
			}
		}
		KeyEvent { code: Char('w'), .. } => {
			if let Some(index) = app.blame_state.selected() {
				let commit_path = app.commit_stack.last().unwrap();
				app.right_panel = Some(git::log_follow(app.repo, &commit_path.path, index, commit_path.commit));
			}
		}
		KeyEvent { code: Char('b'), .. } => {
			if let Some(index) = app.blame_state.selected() {
				let blame = &app.blame[index];
				let parent = app.repo.find_commit(blame.commit)?.parent_id(0)?;
				let line_path = match blame.path.to_owned() {
					Some(p) => p,
					None => app.commit_stack.last().unwrap().path.to_owned(),
				};
				app.blame = git::blame(app.repo, &line_path, parent)?;
				app.blame_state.select(Some(index.min(app.blame.len() - 1)));
				app.commit_stack.push(CommitPath {
					commit: parent,
					path: line_path,
				});
			}
		}
		KeyEvent { code: Char('B'), .. } => {
			if app.commit_stack.len() > 1 {
				app.commit_stack.pop();
				let commit_path = app.commit_stack.last().unwrap();
				app.blame = git::blame(app.repo, &commit_path.path, commit_path.commit)?;
				if let Some(index) = app.blame_state.selected() {
					app.blame_state.select(Some(index.min(app.blame.len() - 1)));
				}
			}
		}
		KeyEvent { code: Char('h'), .. } => app.popup = Some(make_help_text()),
		KeyEvent {
			code: Char('q') | KeyCode::Esc,
			..
		} => {
			if app.right_panel.is_some() {
				app.right_panel = None;
				app.line_history_scroll = 0;
			} else {
				return Ok(false);
			}
		}
		_ => {} // ignored
	};
	Ok(true)
}

fn scroll(app: &mut App, term_size: &Rect, amount: i16) {
	match &app.right_panel {
		Some(line_history) => {
			let max = u16::try_from(line_history.height())
				.unwrap()
				.saturating_sub(term_size.height);
			app.line_history_scroll = app.line_history_scroll.saturating_add_signed(amount).clamp(0, max);
		}
		None => {
			match app.blame_state.selected() {
				Some(index) => {
					let new_index = index.saturating_add_signed(amount.into());
					app.blame_state.select(Some(new_index.clamp(0, app.blame.len() - 1)));
				}
				None => {
					app.blame_state.select(Some(0));
				}
			};
		}
	}
}

fn handle_search(blame: &[git::BlameHunk<'_>], query: &str, blame_state: &mut ListState, forward: bool) {
	let range: Box<dyn Iterator<Item = usize>> = if forward {
		let start = match blame_state.selected() {
			Some(index) => index + 1,
			None => 0,
		};
		Box::new(start..blame.len())
	} else {
		let end = blame_state.selected().unwrap_or(0);
		Box::new((0..end).rev())
	};
	for i in range {
		let line = &blame[i].line.spans.last().unwrap().content;
		if line.contains(query) {
			blame_state.select(Some(i));
			return;
		}
	}
}

fn make_help_text() -> Text<'static> {
	let mut help = vec![
		"h           this help",
		"q  esc      close window",
		"",
		"    moving",
		"",
		"j  ↓        down one line",
		"k  ↑        up one line",
		"d  pgdown   down half a window",
		"u  pgup     up half a window",
		"G  end      to last line",
		"g  home     to first line",
		":123        to line 123",
		"",
		"    search",
		"",
		"/           start searching",
		"enter       search forward",
		"n           repeat search forward",
		"N           repeat search backward",
		"",
		"    git",
		"",
		"enter       show commit",
		"w           trace line through history (git -L)",
		"b           reblame line at parent commit",
		"B           undo/pop blame stack",
	];
	(help.drain(..).map(Line::from).collect::<Vec<_>>()).into()
}

fn ui(frame: &mut Frame, app: &mut App) {
	let constraints = if app.right_panel.is_none() {
		[Constraint::Percentage(100)].as_ref()
	} else {
		[Constraint::Percentage(50), Constraint::Percentage(50)].as_ref()
	};
	let size = Rect::new(
		frame.size().x,
		frame.size().y,
		frame.size().width,
		frame.size().height - 1,
	);
	let chunks = Layout::default()
		.direction(Direction::Horizontal)
		.constraints(constraints)
		.split(size);

	let items: Vec<ListItem> = app.blame.iter().map(|line| ListItem::new(line.line.clone())).collect();
	let commit_path = app.commit_stack.last().unwrap();
	let title = Line::from(vec![
		Span::styled(
			commit_path.commit.to_string(),
			Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
		),
		Span::raw(" "),
		Span::styled(
			commit_path.path.to_str().unwrap(),
			Style::default().fg(Color::LightBlue).add_modifier(Modifier::BOLD),
		),
	]);
	let list = List::new(items)
		.block(Block::default().title(title))
		.highlight_style(Style::default().bg(Color::Indexed(237))); // 232 is black, 255 is white; 237 is dark gray
	frame.render_stateful_widget(list, chunks[0], &mut app.blame_state);

	if let Some(log) = &app.right_panel {
		let paragraph = Paragraph::new(log.clone())
			.block(Block::default().borders(Borders::LEFT))
			.scroll((app.line_history_scroll, 0));
		frame.render_widget(paragraph, chunks[1]);
	}

	let command = match &app.search {
		Some(search) => Some(format!("/{}", search.query.as_str())),
		None => app.line_number.as_ref().map(|ln| format!(":{}", ln)),
	};
	if let Some(cmd_str) = command {
		let paragraph = Paragraph::new(cmd_str).wrap(Wrap { trim: false });
		let size = Rect::new(
			frame.size().x,
			frame.size().y + frame.size().height - 1,
			frame.size().width,
			1,
		);
		frame.render_widget(paragraph, size);
	}

	if let Some(popup) = &app.popup {
		let paragraph = Paragraph::new(popup.clone()).wrap(Wrap { trim: false });
		let area = centered_rect(80, 80, frame.size());
		frame.render_widget(Clear, area);
		frame.render_widget(Block::default().borders(Borders::all()), area);
		frame.render_widget(
			paragraph,
			area.inner(&tui::layout::Margin {
				vertical: 2,
				horizontal: 3,
			}),
		);
	}
}

// from https://github.com/tui-rs-revival/ratatui/blob/main/examples/popup.rs
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
