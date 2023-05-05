use std::fmt::Write;
use std::io::{stdin, stdout};
use std::process::exit;
use unsegen::base::{Terminal};
use unsegen::container::ContainerManager;
use unsegen::input::{Event, Input, Key, NavigateBehavior};

use crate::terminal::{App, Index, Pager};

mod terminal;

fn main() {
	let stdout = stdout();
	let stdin = stdin();
	let stdin = stdin.lock();

	let mut app = App::new();
	for _ in 1..10 {
		writeln!(app.left.buffer, "hi").unwrap();
	}
	let mut manager = ContainerManager::<App>::from_layout(App::one_pane());
	let mut term = Terminal::new(stdout.lock()).unwrap();
	app.draw(&manager, &mut term);

	for input in Input::read_all(stdin) {
		let input = input
			.unwrap()
			.chain(manager.active_container_behavior(&mut app, &mut ()))
			.chain(
				NavigateBehavior::new(&mut manager.navigatable(&mut app))
					.left_on(Key::Char('h'))
					.right_on(Key::Char('l')),
			)
			.finish();
		if let Some(i) = input {
			if let Event::Key(key) = i.event {
				match key {
					Key::Char('\n') => {
						let mut right = Pager::new();
						for _ in 1..10 {
							writeln!(right.buffer, "bye").unwrap();
						}
						app.right = Some(right);
						manager.set_layout(App::two_pane());
						manager.set_active(Index::Right);
					}
					Key::Esc => {
						manager.set_layout(App::one_pane());
					}
					Key::Char('q') => {
						drop(term);
						exit(0);
					}
					_ => {} // ignored
				}
			}
		}
		app.draw(&manager, &mut term);
	}
}
