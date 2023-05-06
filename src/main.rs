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
	app.blame = git::blame(&repo, path);
	let res = terminal::run_app(&mut term, app);

	terminal::teardown(&mut term).unwrap();
	if let Err(err) = res {
		println!("{:?}", err)
	}
}
