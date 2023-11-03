use ansi_to_tui::IntoText;
use git2::{DiffLineType, Oid, Repository};
use std::{
	error,
	path::{Path, PathBuf},
	process, time, vec,
};
use tui::{
	style::{Color, Style},
	text::{Line, Span, Text},
};

#[derive(Debug)]
pub struct BlameHunk<'a> {
	pub spans: Line<'a>,
	pub commit: Oid,
	pub path: Option<PathBuf>,
}

pub fn blame<'a>(
	repo: &'a Repository,
	rel_path: &Path,
	start_commit: Oid,
) -> Result<Vec<BlameHunk<'a>>, Box<dyn error::Error>> {
	let output = process::Command::new("git")
		.args([
			"blame",
			"--porcelain",
			rel_path.to_str().unwrap(),
			&start_commit.to_string(),
		])
		.current_dir(repo.path())
		.output()?;
	if !output.status.success() {
		return Err(std::str::from_utf8(&output.stderr)?.into());
	}
	let blame_output = std::str::from_utf8(&output.stdout)?;
	let blame = crate::git_blame_porcelain::parse_blame_porcelain(blame_output)?;

	let mut out = vec![];
	let now = time::SystemTime::now();
	let duration_formatter = timeago::Formatter::new();
	for b in blame {
		let commit_time = b.info.commit_time;
		let time_display = duration_formatter.convert(now.duration_since(commit_time).unwrap_or_default());
		let mut spans = vec![
			Span::styled(format!("{:.8}", b.commit), Style::default().fg(Color::Yellow)),
			Span::raw(format!(" {}", fmt_width(b.info.author, 12))),
			Span::styled(
				format!(" {}", fmt_width(&time_display, 13)),
				Style::default().fg(Color::LightRed),
			),
		];
		spans.append(&mut format_line_num_and_code(b.line_num, b.code[0]));
		let line_path = b.info.path;
		out.push(BlameHunk {
			spans: Line::from(spans),
			commit: Oid::from_str(b.commit)?,
			path: line_path.map(|p| p.to_owned()),
		});

		for i in 1..b.code.len() {
			let mut spans = vec![Span::raw(" ".repeat(35))];
			let line_num = b.line_num + i32::try_from(i).unwrap();
			spans.append(&mut format_line_num_and_code(line_num, b.code[i]));
			out.push(BlameHunk {
				spans: Line::from(spans),
				commit: Oid::from_str(b.commit)?,
				path: line_path.map(|p| p.to_owned()),
			});
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

fn format_line_num_and_code(line_num: i32, line: &str) -> Vec<Span<'static>> {
	vec![
		Span::styled(format!(" {:4} ", line_num), Style::default().fg(Color::DarkGray)),
		Span::raw(line.replace('\t', "    ")),
	]
}

pub fn show(repo: &Repository, commit_id: Oid) -> Text<'static> {
	let commit = match repo.find_commit(commit_id) {
		Ok(commit) => commit,
		Err(e) => return Text::raw(e.to_string()),
	};
	let parent = match commit.parent(0) {
		Ok(commit) => commit,
		Err(e) => return Text::raw(e.to_string()),
	};
	let parent_tree = match parent.tree() {
		Ok(tree) => tree,
		Err(e) => return Text::raw(e.to_string()),
	};
	let commit_tree = match commit.tree() {
		Ok(tree) => tree,
		Err(e) => return Text::raw(e.to_string()),
	};
	let diff = match repo.diff_tree_to_tree(Some(&parent_tree), Some(&commit_tree), None) {
		Ok(diff) => diff,
		Err(e) => return Text::raw(e.to_string()),
	};

	let author = commit.author();
	let commit_time = commit.time();
	let time = chrono::DateTime::from_timestamp(commit_time.seconds(), 0).unwrap();
	let mut lines = vec![
		Line::from(Span::styled(
			commit.id().to_string(),
			Style::default().fg(Color::Yellow),
		)),
		Line::from(format!(
			"author: {} <{}>",
			author.name().unwrap_or_default(),
			author.email().unwrap_or_default()
		)),
		Line::from(format!("date: {}", time.with_timezone(&chrono::Local))),
		Line::default(),
		Line::from(commit.summary().unwrap_or_default().to_owned()),
		Line::default(),
	];
	if let Some(body) = commit.body() {
		push_lines(&mut lines, body, Color::Reset);
		lines.push(Line::default());
	}
	let diff_cb = |_: git2::DiffDelta, _: Option<git2::DiffHunk>, diff_line: git2::DiffLine| -> bool {
		let content = std::str::from_utf8(diff_line.content()).expect("couldn't decode diff line");
		let sigil = match diff_line.origin_value() {
			DiffLineType::Addition => "+",
			DiffLineType::Deletion => "-",
			DiffLineType::Context => " ",
			_ => "",
		};
		let line = format!("{}{}", sigil, content.replace('\t', "    ").strip_suffix('\n').unwrap());
		let color = match diff_line.origin_value() {
			DiffLineType::FileHeader => Color::Cyan,
			DiffLineType::HunkHeader => Color::Blue,
			DiffLineType::Addition | DiffLineType::AddEOFNL => Color::Green,
			DiffLineType::Deletion | DiffLineType::DeleteEOFNL => Color::Red,
			_ => Color::Reset,
		};
		push_lines(&mut lines, &line, color);
		true
	};
	if let Err(e) = diff.print(git2::DiffFormat::Patch, diff_cb) {
		return Text::raw(e.to_string());
	}
	Text::from(lines)
}

fn push_lines(lines: &mut Vec<Line>, s: &str, color: Color) {
	for line in s.split('\n') {
		lines.push(Line::from(Span::styled(line.to_owned(), Style::default().fg(color))));
	}
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
