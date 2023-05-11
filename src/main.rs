use git2::Repository;
use std::{env, path::Path};

mod git;
mod terminal;

fn main() {
	let args: Vec<String> = env::args().collect();
	if args.len() != 2 {
		println!("usage: {} <filepath>", args[0].rsplit('/').next().unwrap());
		return;
	}
	let abs_path_buf = Path::new(&args[1]).canonicalize().unwrap();
	let abs_path = abs_path_buf.as_path();

	let repo = match Repository::discover(abs_path) {
		Ok(repo) => repo,
		Err(e) => panic!("{}", e),
	};
	let rel_path = abs_path.strip_prefix(repo.workdir().unwrap()).unwrap();

	let head = repo.head().unwrap().target().unwrap();
	let mut app = terminal::App::new(&repo, rel_path, head);
	app.blame = match git::blame(&repo, rel_path, head) {
		Ok(blame) => blame,
		Err(e) => panic!("{}", e),
	};
	let mut term = terminal::setup().unwrap();
	let res = terminal::run_app(&mut term, app);

	terminal::teardown(&mut term);
	if let Err(err) = res {
		println!("{:?}", err)
	}
}
