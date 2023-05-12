use git2::Repository;
use std::{
	env,
	path::{Path, PathBuf},
};

mod git;
mod terminal;

fn main() {
	let args: Vec<String> = env::args().collect();
	if args.len() < 2 || args.len() > 3 {
		println!("usage: {} <filepath> [rev]", args[0].rsplit('/').next().unwrap());
		return;
	}

	let path = Path::new(&args[1]);
	let (repo, abs_path) = match path.canonicalize() {
		Ok(abs_path) => (Repository::discover(&abs_path).unwrap(), abs_path),
		Err(_) => (Repository::open_from_env().unwrap(), path_to_lexical_absolute(path)),
	};
	let rel_path = abs_path.strip_prefix(repo.workdir().unwrap()).unwrap().to_owned();

	let commit = if args.len() == 3 {
		repo.revparse_single(&args[2]).unwrap().id()
	} else {
		repo.head().unwrap().target().unwrap()
	};
	let mut app = terminal::App::new(&repo, &rel_path, commit);
	app.blame = match git::blame(&repo, &rel_path, commit) {
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

fn path_to_lexical_absolute(path: &Path) -> PathBuf {
	// https://internals.rust-lang.org/t/path-to-lexical-absolute/14940
	let mut absolute = if path.is_absolute() {
		PathBuf::new()
	} else {
		std::env::current_dir().unwrap()
	};
	for component in path.components() {
		match component {
			std::path::Component::CurDir => {}
			std::path::Component::ParentDir => {
				absolute.pop();
			}
			component => absolute.push(component.as_os_str()),
		}
	}
	absolute
}
