use std::{
	collections::HashMap,
	error,
	path::Path,
	rc::Rc,
	time::{self, SystemTime},
};

use nom::{
	bytes::complete::{tag, take_till, take_till1, take_until1, take_while1},
	character::complete::line_ending,
	combinator::{opt, peek},
	sequence::{delimited, preceded, terminated, Tuple},
	AsChar, IResult,
};

#[derive(Debug, PartialEq, Eq)]
pub struct BlameLine<'a> {
	pub commit: &'a str,
	pub line_num: i32,
	pub code: Vec<&'a str>,
	pub info: Rc<CommitInfo<'a>>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct CommitInfo<'a> {
	pub author: &'a str,
	pub commit_time: SystemTime,
	pub path: Option<&'a Path>,
}

pub fn parse_blame_porcelain(input: &str) -> Result<Vec<BlameLine<'_>>, Box<dyn error::Error>> {
	let tab = tag::<&str, &str, ()>("\t");
	let mut parse_code = delimited(tab, take_till(is_line_ending), line_ending);

	let mut hunks = vec![];
	let mut commits: HashMap<&str, Rc<CommitInfo>> = HashMap::new();
	let mut remaining = input;
	while !remaining.is_empty() {
		let header;
		(remaining, header) = match parse_header(remaining) {
			Ok(r) => r,
			Err(e) => return Err(Box::new(e.map_input(|es| es.to_owned()))),
		};

		let commit_info = commits.entry(header.commit).or_insert_with(|| {
			let commit_info;
			(remaining, commit_info) = parse_commit_info(remaining).expect("expected commit info");
			Rc::new(commit_info)
		});

		let code_line;
		(remaining, code_line) = parse_code(remaining)?;
		let mut code = vec![code_line];

		for _ in 1..header.group_size {
			(remaining, _) = match parse_header(remaining) {
				Ok(r) => r,
				Err(e) => return Err(Box::new(e.map_input(|es| es.to_owned()))),
			};
			let code_line;
			(remaining, code_line) = parse_code(remaining)?;
			code.push(code_line);
		}

		hunks.push(BlameLine {
			commit: header.commit,
			line_num: header.line_no,
			code,
			info: commit_info.to_owned(),
		});
	}
	Ok(hunks)
}

#[derive(Debug, PartialEq, Eq)]
struct Header<'a> {
	commit: &'a str,
	line_no: i32,
	group_size: i32,
}

fn parse_header(input: &str) -> IResult<&str, Header> {
	let space = tag(" ");
	let commit = terminated(take_until1(" "), &space);
	let orig_line = terminated(take_until1(" "), &space);
	let final_line = take_while1(is_digit);
	let group_size = opt(preceded(&space, take_while1(is_digit)));
	let (remaining, (commit, _, final_line, group_size, _)) =
		(commit, orig_line, final_line, group_size, line_ending).parse(input)?;
	Ok((
		remaining,
		Header {
			commit,
			line_no: final_line.parse().unwrap(),
			group_size: match group_size {
				Some(b) => b.parse().unwrap(),
				None => 1,
			},
		},
	))
}

fn parse_commit_info(input: &str) -> IResult<&str, CommitInfo> {
	let mut parse_line = (
		terminated(take_until1(" "), tag(" ")),
		terminated(take_till1(is_line_ending), line_ending),
	);

	let mut ret = CommitInfo {
		author: "",
		commit_time: SystemTime::UNIX_EPOCH,
		path: None,
	};
	let mut remaining = input;
	while !remaining.is_empty() {
		match peek(tag::<&str, &str, ()>("\t"))(remaining) {
			Ok(_) => {
				return Ok((remaining, ret));
			}
			Err(_) => {
				let (field, value);
				(remaining, (field, value)) = parse_line.parse(remaining)?;
				match field {
					"author" => ret.author = value,
					"committer-time" => {
						let timestamp: u64 = value.parse().unwrap();
						ret.commit_time = make_time(timestamp);
					}
					"filename" => ret.path = Some(Path::new(value)),
					_ => {}
				}
			}
		};
	}
	panic!("couldn't find code line starting with tab")
}

#[inline]
fn is_digit(c: char) -> bool {
	c.is_dec_digit()
}

#[inline]
fn is_line_ending(c: char) -> bool {
	c == '\n' || c == '\r'
}

fn make_time(ts: u64) -> SystemTime {
	SystemTime::UNIX_EPOCH + time::Duration::from_secs(ts)
}

#[cfg(test)]
mod tests {
	use std::{
		path::Path,
		rc::Rc,
		time::{self, SystemTime},
	};

	use crate::git_blame_porcelain::{make_time, parse_blame_porcelain, parse_header, BlameLine, CommitInfo, Header};

	use super::parse_commit_info;

	const TEST_BLAME_OUTPUT: &str = include_str!("../fixtures/test_blame_output");

	#[test]
	fn parse() {
		let result = parse_blame_porcelain(TEST_BLAME_OUTPUT).expect("couldn't parse blame");
		let expected = vec![
			BlameLine {
				commit: "116aa62bf54a39697e25f21d6cf6799f7faa1349",
				line_num: 1,
				code: vec![
					":mod:`gc` --- Garbage Collector interface",
					"=========================================",
					"",
					".. module:: gc",
					"   :synopsis: Interface to the cycle-detecting garbage collector.",
				],
				info: Rc::new(CommitInfo {
					author: "Georg Brandl",
					commit_time: make_time(1187188102),
					path: Some(Path::new("Doc/library/gc.rst")),
				}),
			},
			BlameLine {
				commit: "fa089b9b0b926c04e5d57812b7d7653472787965",
				line_num: 6,
				code: vec![""],
				info: Rc::new(CommitInfo {
					author: "Terry Jan Reedy",
					commit_time: make_time(1465671774),
					path: Some(Path::new("Doc/library/gc.rst")),
				}),
			},
			BlameLine {
				commit: "116aa62bf54a39697e25f21d6cf6799f7faa1349",
				line_num: 7,
				code: vec![
					".. moduleauthor:: Neil Schemenauer <nas@arctrix.com>",
					".. sectionauthor:: Neil Schemenauer <nas@arctrix.com>",
					"",
				],
				info: Rc::new(CommitInfo {
					author: "Georg Brandl",
					commit_time: make_time(1187188102),
					path: Some(Path::new("Doc/library/gc.rst")),
				}),
			},
			BlameLine {
				commit: "fa089b9b0b926c04e5d57812b7d7653472787965",
				line_num: 10,
				code: vec!["--------------"],
				info: Rc::new(CommitInfo {
					author: "Terry Jan Reedy",
					commit_time: make_time(1465671774),
					path: Some(Path::new("Doc/library/gc.rst")),
				}),
			},
		];
		for (actual_line, expected_line) in std::iter::zip(&result, &expected) {
			assert_eq!(actual_line, expected_line);
		}
		assert_eq!(result.len(), expected.len());
	}

	#[test]
	fn header() {
		let result = parse_header(TEST_BLAME_OUTPUT).expect("couldn't parse blame");
		assert_eq!(
			result.1,
			Header {
				commit: "116aa62bf54a39697e25f21d6cf6799f7faa1349",
				line_no: 1,
				group_size: 5,
			}
		);
	}

	#[test]
	fn commit_info() {
		let data = "author raylu
author-mail <mail@fake.tld>
author-time 1234567890
author-tz -1100
committer someguy
committer-mail <fake@mail.tld>
committer-time 9876543210
committer-tz +1100
summary blah blah
previous c92bf83a829956e683a3d6bb1ae65aed74d7b92a Doc/library/gc.rst
filename Doc/library/gc.rst
	line of code
";
		let (remaining, commit_info) = parse_commit_info(data).expect("couldn't parse commit info");
		assert_eq!(remaining, "\tline of code\n");
		assert_eq!(
			commit_info,
			CommitInfo {
				author: "raylu",
				commit_time: SystemTime::UNIX_EPOCH + time::Duration::from_secs(9876543210),
				path: Some(Path::new("Doc/library/gc.rst")),
			}
		);
	}
}
