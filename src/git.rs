use git2::Repository;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use tui::text::Spans;

pub fn blame<'a>(repo: &Repository, path: &Path) -> Vec<Spans<'a>> {
	let repo_path = repo.workdir().unwrap();
	let rel_path = path.strip_prefix(repo_path).unwrap();
	let blame = match repo.blame_file(rel_path, None) {
		Ok(blame) => blame,
		Err(e) => panic!("{}", e),
	};
	let mut lines = BufReader::new(File::open(path).unwrap()).lines();
	let mut out = vec![];
	for b in blame.iter() {
		out.push(Spans::from(format!(
			"{} {} {}",
			b.final_signature().name().unwrap_or_default(),
			b.final_commit_id(),
			lines.next().unwrap().unwrap()
		)));
		for _ in 1..b.lines_in_hunk() {
			out.push(Spans::from(format!("\t{}", lines.next().unwrap().unwrap())));
		}
	}
	out
}
