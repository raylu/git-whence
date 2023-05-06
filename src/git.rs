use git2::Repository;
use std::{
	error,
	fs::File,
	io::{BufRead, BufReader},
	path::Path,
};
use tui::text::Spans;

pub fn blame<'a>(repo: &Repository, path: &Path) -> Result<Vec<Spans<'a>>, Box<dyn error::Error>> {
	let repo_path = repo.workdir().unwrap();
	let rel_path = path.strip_prefix(repo_path).unwrap();
	let blame = repo.blame_file(rel_path, None)?;
	let mut lines = BufReader::new(File::open(path)?).lines();
	let mut out = vec![];
	for b in blame.iter() {
		out.push(Spans::from(format!(
			"{} {} {}",
			b.final_signature().name().unwrap_or_default(),
			b.final_commit_id(),
			lines.next().unwrap()?
		)));
		for _ in 1..b.lines_in_hunk() {
			out.push(Spans::from(format!("\t{}", lines.next().unwrap()?)));
		}
	}
	Ok(out)
}
