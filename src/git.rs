use git2::Repository;
use std::fmt::Write;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use unsegen::widget::builtin::LogViewer;

pub fn blame(repo: &Repository, path: &Path, out: &mut LogViewer) {
	let repo_path = repo.workdir().unwrap();
	let rel_path = path.strip_prefix(repo_path).unwrap();
	let blame = match repo.blame_file(rel_path, None) {
		Ok(blame) => blame,
		Err(e) => panic!("{}", e),
	};
	let mut lines = BufReader::new(File::open(path).unwrap()).lines();
	for b in blame.iter() {
		write!(
			out,
			"{} {}",
			b.final_signature().name().unwrap_or_default(),
			b.final_commit_id()
		)
		.unwrap();
		writeln!(out, " {}", lines.next().unwrap().unwrap()).unwrap();
		for _ in 1..b.lines_in_hunk() {
			writeln!(out, "\t{}", lines.next().unwrap().unwrap()).unwrap();
		}
	}
}
