use ansi_to_tui::IntoText;
use git2::{BlameOptions, Oid, Repository};
use std::{
	error,
	io::{BufRead, BufReader},
	path::Path,
	process,
};
use tui::{
	style::{Color, Style},
	text::{Span, Spans, Text},
};

#[derive(Debug)]
pub struct BlameLine {
	pub spans: Spans<'static>,
	pub commit: Oid,
}

pub fn blame(repo: &Repository, path: &Path, start_commit: Oid) -> Result<Vec<BlameLine>, Box<dyn error::Error>> {
	let repo_path = repo.workdir().unwrap();
	let rel_path = path.strip_prefix(repo_path).unwrap();

	let mut opts = BlameOptions::default();
	opts.newest_commit(start_commit);
	let blame = repo.blame_file(rel_path, Some(&mut opts))?;

	let blob = repo
		.find_commit(start_commit)?
		.tree()?
		.get_path(rel_path)?
		.to_object(repo)?;
	let mut lines = BufReader::new(blob.as_blob().unwrap().content()).lines();

	let mut out = vec![];
	let mut line_num: usize = 1;
	for b in blame.iter() {
		let mut commit = b.final_commit_id().to_string();
		commit.truncate(8);
		let author = format!(" {:12}", b.final_signature().name().unwrap_or_default());
		let spans = Spans::from(vec![
			Span::styled(commit, Style::default().fg(Color::Yellow)),
			Span::raw(author),
			Span::styled(format!(" {:4} ", line_num), Style::default().fg(Color::DarkGray)),
			Span::raw(lines.next().unwrap()?.replace('\t', "    ")),
		]);
		out.push(BlameLine {
			spans,
			commit: b.final_commit_id(),
		});
		line_num += 1;
		for _ in 1..b.lines_in_hunk() {
			let spans = Spans::from(vec![
				Span::raw(" ".repeat(21)),
				Span::styled(format!(" {:4} ", line_num), Style::default().fg(Color::DarkGray)),
				Span::raw(lines.next().unwrap()?.replace('\t', "    ")),
			]);
			line_num += 1;
			out.push(BlameLine {
				spans,
				commit: b.final_commit_id(),
			});
		}
	}
	Ok(out)
}

pub fn log_follow(repo: &Repository, path: &Path, line_num: usize, start_commit: Oid) -> Text<'static> {
	let repo_path = repo.workdir().unwrap();
	let rel_path = path.strip_prefix(repo_path).unwrap();
	let output = process::Command::new("git")
		.args([
			"log",
			"--color=always",
			"--abbrev-commit",
			"--decorate",
			"--date=local",
			"--find-copies",
			"-L",
			&format!("{},{}:{}", line_num + 1, line_num + 1, rel_path.display()),
			&start_commit.to_string(),
		])
		.current_dir(repo_path)
		.output();

	let buf = match output {
		Ok(o) => {
			if o.status.success() {
				o.stdout
			} else {
				o.stderr
			}
		}
		Err(e) => {
			return Text::raw(e.to_string());
		}
	};
	match buf.into_text() {
		Ok(t) => t,
		Err(e) => Text::raw(format!("ansi_to_tui:\n{}", e)),
	}
}
