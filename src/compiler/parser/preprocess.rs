use std::fs;
use std::path::{Path, PathBuf};

use chumsky::prelude::*;

use super::object;
use crate::compiler::parser::MyExtra;

#[derive(Debug, Clone, PartialEq)]
pub struct IncludeParams {
    pub n_whitespace: usize,
    pub file_name: String,
    pub block_name: Option<String>,
    pub language: String,
    pub min_level: Option<usize>,
    pub lines: (Option<usize>, Option<usize>),
    pub only_contents: bool,
}

pub(crate) fn include_parser<'a, C: 'a>() -> impl Parser<'a, &'a str, IncludeParams, MyExtra<'a, C>>
{
    let file_name = object::whitespaces_g1().ignore_then(choice((
        none_of(" \t\n")
            .repeated()
            .at_least(1)
            .to_slice()
            .delimited_by(just('"'), just('"')),
        none_of(" \t\n").repeated().at_least(1).to_slice(),
    )));

    let maybe_block_name = object::whitespaces_g1()
        .ignore_then(none_of(" \t\n:").repeated().at_least(1).to_slice())
        .or_not();

    let maybe_language = object::whitespaces_g1()
        .ignore_then(
            any()
                .filter(|c: &char| c.is_alphabetic() || matches!(c, '-'))
                .repeated()
                .at_least(1)
                .to_slice(),
        )
        .or_not();

    let lines = object::whitespaces_g1().ignore_then(just(":lines")).then(
        object::whitespaces_g1().ignore_then(
            one_of("-0123456789")
                .repeated()
                .at_least(1)
                .to_slice()
                .delimited_by(just('"'), just('"')),
        ),
    );

    let min_level = object::whitespaces_g1()
        .ignore_then(just(":minlevel"))
        .then(
            object::whitespaces_g1()
                .ignore_then(one_of("0123456789").repeated().at_least(1).to_slice()),
        );

    let only_contents = object::whitespaces_g1()
        .ignore_then(just(":only-contents"))
        .then(
            object::whitespaces_g1().ignore_then(
                any()
                    .filter(|c: &char| c.is_alphabetic())
                    .repeated()
                    .at_least(1)
                    .to_slice(),
            ),
        );

    let named_args = choice((lines, min_level, only_contents))
        .repeated()
        .collect::<Vec<_>>();

    object::whitespaces()
        .then(object::just_case_insensitive("#+include:"))
        .then(file_name)
        .then(maybe_block_name)
        .then(maybe_language)
        .then(named_args)
        .then_ignore(object::whitespaces())
        .map(
            move |(
                ((((whitespaces, _), file_name), maybe_block_name), maybe_language),
                named_args,
            )| {
                let n_whitespace = whitespaces.len();
                let (mut min_level, mut only_contents, mut lines) = (None, false, (None, None));
                for (k, v) in named_args {
                    match k {
                        ":only-contents" => {
                            if v.to_lowercase() != "nil" {
                                only_contents = true;
                            }
                        }
                        ":minlevel" => {
                            min_level = Some(v.parse::<usize>().expect("a number"));
                        }
                        ":lines" => {
                            let (mut start_line, mut end_line) = (None, None);
                            if v.starts_with("-") {
                                end_line = Some(
                                    v.split("-")
                                        .last()
                                        .expect("number")
                                        .parse::<usize>()
                                        .unwrap(),
                                );
                            } else if v.ends_with("-") {
                                start_line = Some(
                                    v.split("-")
                                        .next()
                                        .expect("number")
                                        .parse::<usize>()
                                        .unwrap(),
                                );
                            } else {
                                let ab = v.split("-").into_iter().collect::<Vec<_>>();
                                if ab.len() == 2 {
                                    start_line = Some(ab[0].parse::<usize>().expect("a number"));
                                    end_line = Some(ab[1].parse::<usize>().expect("a number"));
                                }
                            }

                            lines = (start_line, end_line);
                        }
                        _ => {}
                    }
                }

                IncludeParams {
                    n_whitespace,
                    file_name: file_name.to_string(),
                    block_name: maybe_block_name.map(|e| e.to_string()),
                    language: maybe_language.map_or("org".to_string(), |e| e.to_string()),
                    lines,
                    min_level,
                    only_contents,
                }
            },
        )
}

pub struct IncludePreProcessor {
    pub input_file: PathBuf, // the file which to preprocess
}

use std::borrow::Cow;
impl IncludePreProcessor {
    pub(crate) fn parse<'a>(&self, line: &'a str) -> Cow<'a, str> {
        let line_ = line.trim();
        if !line_.starts_with("#+INCLUDE:") && !line_.starts_with("#+include:") {
            return Cow::Borrowed(line);
        }

        let (maybe_output, errors) = include_parser::<()>().parse(line).into_output_errors();

        if let Some(include_params) = maybe_output {
            let path = Path::new(&include_params.file_name);
            let path = if !path.is_absolute() {
                self.input_file
                    .parent()
                    .expect("should have parent directory")
                    .join(path)
            } else {
                path.to_path_buf()
            };
            if !path.exists() {
                tracing::error!("parse include failed, cound't found {}", path.display());
                return Cow::Borrowed(line);
            }

            let content = &fs::read_to_string(path).expect("read failed");
            let lines: Vec<&str> = content.lines().collect();

            let (start_line, end_line) = (
                include_params.lines.0.unwrap_or(0),
                include_params.lines.1.unwrap_or(lines.len()),
            );
            let start_line = start_line.max(0);
            let end_line = end_line.min(lines.len());

            if start_line > end_line {
                tracing::error!("parse include failed: {start_line} > {end_line}");
                return Cow::Borrowed(line);
            }

            // keep src block with same indented with "spaces before #+include"
            let prefix_whitespaces = " ".repeat(include_params.n_whitespace);
            let ans = lines[start_line - 1..end_line - 1]
                .iter()
                .copied()
                .map(|e| format!("{prefix_whitespaces}{e}"))
                .collect::<Vec<String>>()
                .join("\n");

            let ans = if include_params.language != "org" {
                vec![
                    format!(
                        "{}#+BEGIN_SRC {}",
                        prefix_whitespaces, include_params.language
                    ),
                    ans,
                    format!("{}#+END_SRC", prefix_whitespaces),
                ]
                .join("\n")
            } else {
                ans
            };

            let ans = Cow::Owned(ans);

            tracing::debug!("include:{}", ans);
            ans
        } else {
            tracing::error!("parse include failed: {:?}", errors);
            return Cow::Borrowed(line);
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_include_01() {
        let parser = include_parser::<()>();
        let input = r##"#+INCLUDE: "~/.emacs" src emacs-lisp"##;
        let expected_output = r##"IncludeParams {
    file_name: "\"~/.emacs\"",
    block_name: Some(
        "src",
    ),
    language: "emacs-lisp",
    min_level: None,
    lines: (
        None,
        None,
    ),
    only_contents: false,
}"##;
        let (maybe_output, errors) = parser.parse(input).into_output_errors();
        if let Some(output) = maybe_output {
            assert_eq!(format!("{:#?}", output), expected_output);
        } else {
            panic!("{:?}", errors);
        }
    }

    #[test]
    fn test_include_02() {
        let parser = include_parser::<()>();
        let input = r##"#+INCLUDE: "~/my-book/chapter2.org" :minlevel 1"##;
        let expected_output = r##"IncludeParams {
    file_name: "\"~/my-book/chapter2.org\"",
    block_name: None,
    language: "org",
    min_level: Some(
        1,
    ),
    lines: (
        None,
        None,
    ),
    only_contents: false,
}"##;
        let (maybe_output, errors) = parser.parse(input).into_output_errors();
        if let Some(output) = maybe_output {
            assert_eq!(format!("{:#?}", output), expected_output);
        } else {
            panic!("{:?}", errors);
        }
    }

    #[test]
    fn test_include_03() {
        let parser = include_parser::<()>();
        let input = r##"#+INCLUDE: "~/.emacs" :lines "5-10""##;
        let expected_output = r##"IncludeParams {
    file_name: "\"~/.emacs\"",
    block_name: None,
    language: "org",
    min_level: None,
    lines: (
        Some(
            5,
        ),
        Some(
            10,
        ),
    ),
    only_contents: false,
}"##;
        let (maybe_output, errors) = parser.parse(input).into_output_errors();
        if let Some(output) = maybe_output {
            assert_eq!(format!("{:#?}", output), expected_output);
        } else {
            panic!("{:?}", errors);
        }
    }

    #[test]
    fn test_include_04() {
        let parser = include_parser::<()>();
        let input = r##"#+INCLUDE: "~/.emacs" :lines "-10" "##;
        let expected_output = r##"IncludeParams {
    file_name: "\"~/.emacs\"",
    block_name: None,
    language: "org",
    min_level: None,
    lines: (
        None,
        Some(
            10,
        ),
    ),
    only_contents: false,
}"##;
        let (maybe_output, errors) = parser.parse(input).into_output_errors();
        if let Some(output) = maybe_output {
            assert_eq!(format!("{:#?}", output), expected_output);
        } else {
            panic!("{:?}", errors);
        }
    }

    #[test]
    fn test_include_05() {
        let parser = include_parser::<()>();
        let input = r##"#+INCLUDE: "~/.emacs" :lines "10-" "##;
        let expected_output = r##"IncludeParams {
    file_name: "\"~/.emacs\"",
    block_name: None,
    language: "org",
    min_level: None,
    lines: (
        Some(
            10,
        ),
        None,
    ),
    only_contents: false,
}"##;
        let (maybe_output, errors) = parser.parse(input).into_output_errors();
        if let Some(output) = maybe_output {
            assert_eq!(format!("{:#?}", output), expected_output);
        } else {
            panic!("{:?}", errors);
        }
    }

    #[test]
    fn test_include_06() {
        let parser = include_parser::<()>();
        let input = r##"#+INCLUDE: "./paper.org::*conclusion" :lines "1-20" "##;
        let expected_output = r##"IncludeParams {
    file_name: "\"./paper.org::*conclusion\"",
    block_name: None,
    language: "org",
    min_level: None,
    lines: (
        Some(
            1,
        ),
        Some(
            20,
        ),
    ),
    only_contents: false,
}"##;
        let (maybe_output, errors) = parser.parse(input).into_output_errors();
        if let Some(output) = maybe_output {
            assert_eq!(format!("{:#?}", output), expected_output);
        } else {
            panic!("{:?}", errors);
        }
    }

    #[test]
    fn test_include_07() {
        let parser = include_parser::<()>();
        let input = r##"#+INCLUDE: "./paper.org::#theory" :only-contents t"##;
        let expected_output = r##"IncludeParams {
    file_name: "\"./paper.org::#theory\"",
    block_name: None,
    language: "org",
    min_level: None,
    lines: (
        None,
        None,
    ),
    only_contents: true,
}"##;
        let (maybe_output, errors) = parser.parse(input).into_output_errors();
        if let Some(output) = maybe_output {
            assert_eq!(format!("{:#?}", output), expected_output);
        } else {
            panic!("{:?}", errors);
        }
    }
}
