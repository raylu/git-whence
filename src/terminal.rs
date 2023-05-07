use crossterm::{
	event::{
		self, DisableMouseCapture, EnableMouseCapture, Event,
		KeyCode::{self, Char},
		KeyEvent,
	},
	execute,
	terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use git2::Repository;
use std::{
	error::Error,
	io::{self, Stdout},
	path::Path,
};
use tui::{
	backend::{Backend, CrosstermBackend},
	layout::{Alignment, Constraint, Direction, Layout},
	style::{Color, Style},
	text::Text,
	widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
	Frame, Terminal,
};

use crate::git;

pub struct App<'a> {
	pub blame: Vec<git::BlameLine>,
	blame_state: ListState,
	repo: &'a Repository,
	filepath: &'a Path,
	line_history: Option<Text<'static>>,
}

impl App<'_> {
	pub fn new<'a>(repo: &'a Repository, filepath: &'a Path) -> App<'a> {
		App {
			blame: vec![],
			blame_state: ListState::default(),
			repo,
			filepath,
			line_history: None,
		}
	}
}

type CrosstermTerm = Terminal<CrosstermBackend<Stdout>>;

pub fn setup() -> Result<CrosstermTerm, Box<dyn Error>> {
	enable_raw_mode()?;
	let mut stdout = io::stdout();
	execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
	let backend = CrosstermBackend::new(stdout);
	Ok(Terminal::new(backend)?)
}

pub fn run_app(terminal: &mut CrosstermTerm, mut app: App) -> Result<(), Box<dyn Error>> {
	loop {
		terminal.draw(|f| ui(f, &mut app))?;

		if let Event::Key(key) = event::read()? {
			match key {
				KeyEvent {
					code: Char('j') | KeyCode::Down,
					..
				} => match app.blame_state.selected() {
					Some(index) => {
						if index < app.blame.len() - 1 {
							app.blame_state.select(Some(index + 1));
						}
					}
					None => {
						app.blame_state.select(Some(0));
					}
				},
				KeyEvent {
					code: Char('k') | KeyCode::Up,
					..
				} => match app.blame_state.selected() {
					Some(index) => {
						if index > 0 {
							app.blame_state.select(Some(index - 1));
						}
					}
					None => {
						app.blame_state.select(Some(0));
					}
				},
				KeyEvent {
					code: KeyCode::Enter, ..
				} => {
					if let Some(index) = app.blame_state.selected() {
						app.line_history = Some(git::log_follow(app.repo, app.filepath, index));
					}
				}
				KeyEvent { code: Char('b'), .. } => {
					if let Some(index) = app.blame_state.selected() {
						let parent = app.repo.find_commit(app.blame[index].commit)?.parent_id(0)?;
						app.blame = git::blame(&app.repo, app.filepath, Some(parent))?;
						app.blame_state = ListState::default();
					}
				}
				KeyEvent {
					code: Char('q') | KeyCode::Esc,
					..
				} => {
					if app.line_history.is_some() {
						app.line_history = None
					} else {
						return Ok(());
					}
				}
				_ => {} // ignored
			}
		}
	}
}

pub fn teardown(terminal: &mut CrosstermTerm) {
	_ = disable_raw_mode();
	_ = execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture);
	_ = terminal.show_cursor();
}

fn ui<B: Backend>(frame: &mut Frame<B>, app: &mut App) {
	let constraints: &[Constraint];
	if app.line_history.is_none() {
		constraints = [Constraint::Percentage(100)].as_ref();
	} else {
		constraints = [Constraint::Percentage(50), Constraint::Percentage(50)].as_ref();
	}
	let chunks = Layout::default()
		.direction(Direction::Horizontal)
		.constraints(constraints)
		.split(frame.size());

	let items: Vec<ListItem> = app.blame.iter().map(|line| ListItem::new(line.spans.clone())).collect();
	let list = List::new(items).highlight_style(Style::default().bg(Color::DarkGray));
	frame.render_stateful_widget(list, chunks[0], &mut app.blame_state);

	if let Some(log) = &app.line_history {
		let paragraph = Paragraph::new(log.clone())
			.block(Block::default().borders(Borders::LEFT))
			.alignment(Alignment::Left);
		frame.render_widget(paragraph, chunks[1]);
	}
}
