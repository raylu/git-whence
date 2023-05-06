use git2::Repository;
use std::{env, path::Path};

mod git;
mod terminal;

fn main() {
	let args: Vec<String> = env::args().collect();
	if args.len() != 2 {
		println!("usage: {} <filepath>", args[0].rsplitn(2, "/").next().unwrap());
	}
	let path = Path::new(&args[1]);

	let repo = match Repository::discover(path) {
		Ok(repo) => repo,
		Err(e) => panic!("{}", e),
	};

	let mut term = terminal::setup().unwrap();
	let mut app = terminal::App::new();
	app.blame = match git::blame(&repo, path) {
		Ok(blame) => blame,
		Err(e) => panic!("{}", e),
	};
	let res = terminal::run_app(&mut term, app);

	terminal::teardown(&mut term);
	if let Err(err) = res {
		println!("{:?}", err)
	}
}
