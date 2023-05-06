use crossterm::{
	event::{
		self, DisableMouseCapture, EnableMouseCapture, Event,
		KeyCode::{self, Char},
		KeyEvent,
	},
	execute,
	terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
	error::Error,
	io::{self, Stdout},
};
use tui::{
	backend::{Backend, CrosstermBackend},
	layout::{Alignment, Constraint, Direction, Layout},
	style::{Color, Style},
	text::Spans,
	widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
	Frame, Terminal,
};

pub struct App<'a> {
	pub blame: Vec<Spans<'a>>,
	blame_state: ListState,
}

impl App<'_> {
	pub fn new<'a>() -> App<'a> {
		App {
			blame: vec![],
			blame_state: ListState::default(),
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

pub fn run_app(terminal: &mut CrosstermTerm, mut app: App) -> io::Result<()> {
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
				KeyEvent { code: Char('q'), .. } => {
					return Ok(());
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
	let size = frame.size();
	let chunks = Layout::default()
		.direction(Direction::Horizontal)
		.constraints([Constraint::Percentage(80), Constraint::Percentage(20)].as_ref())
		.split(size);

	let items: Vec<ListItem> = app.blame.iter().map(|line| ListItem::new(line.clone())).collect();
	let list = List::new(items)
		.block(Block::default().borders(Borders::RIGHT))
		.highlight_style(Style::default().bg(Color::DarkGray));
	frame.render_stateful_widget(list, chunks[0], &mut app.blame_state);

	let paragraph = Paragraph::new("").block(Block::default()).alignment(Alignment::Left);
	frame.render_widget(paragraph, chunks[1]);
}
