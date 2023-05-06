use git2::Repository;
use std::env;
use std::fmt::Write;
use std::io::{stdin, stdout};
use std::path::Path;
use std::process::exit;
use unsegen::base::Terminal;
use unsegen::container::ContainerManager;
use unsegen::input::{Event, Input, Key, NavigateBehavior, Scrollable};

use crate::terminal::{App, Index, Pager};

mod git;
mod terminal;

fn main() {
	let args: Vec<String> = env::args().collect();
	if args.len() != 2 {
		println!("usage: {} <filepath>", args[0].rsplitn(2, "/").next().unwrap());
		return;
	}
	let path = Path::new(&args[1]);

	let repo = match Repository::discover(path) {
		Ok(repo) => repo,
		Err(e) => panic!("{}", e),
	};

	let stdout = stdout();
	let stdin = stdin();
	let stdin = stdin.lock();

	let mut app = App::new();
	let mut manager = ContainerManager::<App>::from_layout(App::one_pane());
	let mut term = Terminal::new(stdout.lock()).unwrap();

	git::blame(&repo, path, &mut app.left.buffer);
	app.left.buffer.scroll_to_beginning().unwrap();
	let height = term.create_root_window().get_height().raw_value();
	for _ in 1..height {
		app.left.buffer.scroll_forwards().unwrap();
	}

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
