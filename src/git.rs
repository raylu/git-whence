use ansi_to_tui::IntoText;
use git2::{BlameOptions, Oid, Repository};
use std::{
	error,
	io::{BufRead, BufReader},
	path::{Path, PathBuf},
	process, time,
};
use tui::{
	style::{Color, Style},
	text::{Span, Spans, Text},
};

#[derive(Debug)]
pub struct BlameLine<'a> {
	pub spans: Spans<'a>,
	pub commit: Oid,
	pub path: Option<PathBuf>,
}

pub fn blame<'a>(
	repo: &'a Repository,
	rel_path: &Path,
	start_commit: Oid,
) -> Result<Vec<BlameLine<'a>>, Box<dyn error::Error>> {
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
	let now = time::SystemTime::now();
	let duration_formatter = timeago::Formatter::new();
	for b in blame.iter() {
		let signature = b.final_signature();
		let commit_time = time::UNIX_EPOCH + time::Duration::from_secs(signature.when().seconds().try_into().unwrap());
		let time_display = duration_formatter.convert(now.duration_since(commit_time).unwrap_or_default());
		let mut spans = vec![
			Span::styled(
				format!("{:.8}", b.final_commit_id()),
				Style::default().fg(Color::Yellow),
			),
			Span::raw(format!(" {}", fmt_width(signature.name().unwrap_or_default(), 12))),
			Span::styled(
				format!(" {}", fmt_width(&time_display, 13)),
				Style::default().fg(Color::LightRed),
			),
		];
		spans.append(&mut format_line_num_and_code(line_num, &lines.next().unwrap()?));
		let line_path = b.path().map(|p| p.to_owned());
		out.push(BlameLine {
			spans: Spans::from(spans),
			commit: b.final_commit_id(),
			path: line_path.clone(),
		});
		line_num += 1;
		for _ in 1..b.lines_in_hunk() {
			let mut spans = vec![Span::raw(" ".repeat(35))];
			spans.append(&mut format_line_num_and_code(line_num, &lines.next().unwrap()?));
			out.push(BlameLine {
				spans: Spans::from(spans),
				commit: b.final_commit_id(),
				path: line_path.clone(),
			});
			line_num += 1;
		}
	}
	Ok(out)
}

fn fmt_width(s: &str, width: usize) -> String {
	let mut out = String::new();
	match s.char_indices().nth(width) {
		None => out.push_str(s),
		Some((i, _)) => out.push_str(&s[..i]),
	};
	if out.len() < width {
		out.push_str(&" ".repeat(width - out.len()));
	}
	out
}

fn format_line_num_and_code(line_num: usize, line: &str) -> Vec<Span<'static>> {
	vec![
		Span::styled(format!(" {:4} ", line_num), Style::default().fg(Color::DarkGray)),
		Span::raw(line.replace('\t', "    ")),
	]
}

pub fn log_follow(repo: &Repository, rel_path: &Path, line_num: usize, start_commit: Oid) -> Text<'static> {
	let repo_path = repo.workdir().unwrap();
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
