use crossterm::{
	event::{
		self, Event,
		KeyCode::{self, Char},
		KeyEvent,
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
	backend::{Backend, CrosstermBackend},
	layout::{Alignment, Constraint, Direction, Layout, Rect},
	style::{Color, Modifier, Style},
	text::{Span, Spans, Text},
	widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
	Frame, Terminal,
};

use crate::git;

pub struct App<'a> {
	pub blame: Vec<git::BlameHunk<'a>>,
	blame_state: ListState,
	repo: &'a Repository,
	commit_stack: Vec<CommitPath>,       // pushed by `b`, popped by `B`
	line_history: Option<Text<'static>>, // output of git -L
	line_history_scroll: u16,
	popup: Option<Text<'static>>,
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
			line_history: None,
			line_history_scroll: 0,
			popup: None,
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
		} => match &app.line_history {
			Some(_) => app.line_history_scroll = 0,
			None => app.blame_state.select(Some(0)),
		},
		KeyEvent { code: Char('G'), .. } | KeyEvent { code: KeyCode::End, .. } => match &app.line_history {
			Some(line_history) => {
				app.line_history_scroll = u16::try_from(line_history.height())
					.unwrap()
					.saturating_sub(term_size.height)
			}
			None => app.blame_state.select(Some(app.blame.len() - 1)),
		},
		// other interactions
		KeyEvent {
			code: KeyCode::Enter, ..
		} => {
			if let Some(index) = app.blame_state.selected() {
				let commit_path = app.commit_stack.last().unwrap();
				app.line_history = Some(git::log_follow(app.repo, &commit_path.path, index, commit_path.commit));
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
			if app.line_history.is_some() {
				app.line_history = None;
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
	match &app.line_history {
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
		"",
		"    git",
		"",
		"enter       trace line through history (git -L)",
		"b           reblame line at parent commit",
		"B           undo/pop blame stack",
	];
	(help.drain(..).map(Spans::from).collect::<Vec<_>>()).into()
}

fn ui<B: Backend>(frame: &mut Frame<B>, app: &mut App) {
	let constraints = if app.line_history.is_none() {
		[Constraint::Percentage(100)].as_ref()
	} else {
		[Constraint::Percentage(50), Constraint::Percentage(50)].as_ref()
	};
	let chunks = Layout::default()
		.direction(Direction::Horizontal)
		.constraints(constraints)
		.split(frame.size());

	let items: Vec<ListItem> = app.blame.iter().map(|line| ListItem::new(line.spans.clone())).collect();
	let commit_path = app.commit_stack.last().unwrap();
	let title = Spans::from(vec![
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

	if let Some(log) = &app.line_history {
		let paragraph = Paragraph::new(log.clone())
			.block(Block::default().borders(Borders::LEFT))
			.alignment(Alignment::Left)
			.scroll((app.line_history_scroll, 0));
		frame.render_widget(paragraph, chunks[1]);
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
