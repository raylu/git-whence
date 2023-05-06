use crossterm::{
	event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode::Char, KeyEvent},
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
	style::{Modifier, Style},
	text::{Span, Spans},
	widgets::{Block, Borders, Paragraph},
	Frame, Terminal,
};

pub struct App<'a> {
	pub blame: Vec<Spans<'a>>,
	scroll: u16,
}

impl App<'_> {
	pub fn new<'a>() -> App<'a> {
		App {
			blame: vec![],
			scroll: 0,
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
		terminal.draw(|f| ui(f, &app))?;

		if let Event::Key(key) = event::read()? {
			match key {
				KeyEvent { code: Char('j'), .. } => {
					app.scroll += 1;
				}
				KeyEvent { code: Char('k'), .. } => {
					if app.scroll > 0 {
						app.scroll -= 1;
					}
				}
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

fn ui<B: Backend>(frame: &mut Frame<B>, app: &App) {
	let size = frame.size();
	let chunks = Layout::default()
		.direction(Direction::Horizontal)
		.constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
		.split(size);

	let block = Block::default()
		.borders(Borders::ALL)
		.title(Span::styled("block", Style::default().add_modifier(Modifier::BOLD)));
	let paragraph = Paragraph::new(app.blame.clone())
		.block(block.clone())
		.alignment(Alignment::Left)
		.scroll((app.scroll, 0));
	frame.render_widget(paragraph, chunks[0]);

	let paragraph = Paragraph::new("")
		.block(block)
		.alignment(Alignment::Left)
		.scroll((app.scroll, 0));
	frame.render_widget(paragraph, chunks[1]);
}
