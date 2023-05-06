use git2::Repository;
use std::{
	error,
	fs::File,
	io::{BufRead, BufReader},
	path::Path,
};
use tui::{
	style::{Color, Style},
	text::{Span, Spans},
};

pub fn blame(repo: &Repository, path: &Path) -> Result<Vec<Spans<'static>>, Box<dyn error::Error>> {
	let repo_path = repo.workdir().unwrap();
	let rel_path = path.strip_prefix(repo_path).unwrap();
	let blame = repo.blame_file(rel_path, None)?;
	let mut lines = BufReader::new(File::open(path)?).lines();
	let mut out = vec![];
	let mut line_no = 1;
	for b in blame.iter() {
		let mut commit = b.final_commit_id().to_string();
		commit.truncate(8);
		let author = format!(" {:12}", b.final_signature().name().unwrap_or_default());
		out.push(Spans::from(vec![
			Span::styled(commit, Style::default().fg(Color::Yellow)),
			Span::raw(author),
			Span::styled(format!(" {:4} ", line_no), Style::default().fg(Color::DarkGray)),
			Span::raw(lines.next().unwrap()?),
		]));
		line_no += 1;
		for _ in 1..b.lines_in_hunk() {
			out.push(Spans::from(vec![
				Span::raw(" ".repeat(21)),
				Span::styled(format!(" {:4} ", line_no), Style::default().fg(Color::DarkGray)),
				Span::raw(lines.next().unwrap()?),
			]));
			line_no += 1;
		}
	}
	Ok(out)
}
