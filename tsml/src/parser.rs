// Slightly big parser, yet to be documented
//
use std::{collections::HashMap, fmt, path::PathBuf, result};

use crate::{flags::Flags, lexer::SpannedLexToken, FileTree, GroupsMap, LexToken};

type Stack<T> = Vec<T>;

enum ParserState {
    Clear,
    Busy,
}

#[derive(Debug)]
pub struct ParserError {
    line: usize,
    column: usize,
    kind: ParserErrorKind,
}

impl ParserError {
    pub(crate) fn new(line: usize, column: usize, kind: ParserErrorKind) -> Self {
        ParserError { line, column, kind }
    }
}

#[derive(Debug)]
pub enum ParserErrorKind {
    BracketUnclosed,
    BracketUnexpectedClose,
    BracketUnexpectedOpen,
    CommasOutsideOfBrackets,
    MissingSymlinkTarget,
    FlagAfterFlag,
}

pub type ParserResult<T> = result::Result<T, ParserError>;

fn update_map_group(map: &mut GroupsMap, group: String, files: &mut Stack<FileTree>) {
    let vec = map.entry(group).or_default();
    vec.append(files);
}

pub fn parse_tokens(
    spanned_tokens: Vec<SpannedLexToken>,
    original_text: &str,
) -> ParserResult<(GroupsMap, Vec<String>)> {
    let mut map = GroupsMap::new();

    let mut current_line = 1;
    let mut current_line_start_index = 0;

    let mut file_stack: Stack<FileTree> = Stack::new();
    let mut quantity_stack: Stack<usize> = vec![0];
    let mut read_state = ParserState::Clear;
    let mut current_group = String::from("main");
    let mut already_read_some_lmao = false;
    let mut brackets_open_position = vec![];

    let mut tokens_iter = spanned_tokens.into_iter().peekable();
    let mut depth = 0;

    let mut group_flags = Vec::<String>::new();
    let mut last_flags = Vec::<String>::new();

    let mut group_order = vec!["main".to_string()];
    let mut groups_seen = HashMap::<String, ()>::new();
    groups_seen.insert("main".to_string(), ());

    // let mut pending_flags: Vec<String> = vec![];

    while let Some((token, range)) = tokens_iter.next() {
        let current_column = range.start - current_line_start_index;
        match &token {
            LexToken::Value(value) => {
                *quantity_stack.last_mut().unwrap() += 1;

                if let ParserState::Busy = read_state {
                    panic!("busy when read this token: '{:?}'", token);
                    // panic!("{:?}", range);
                }
                read_state = ParserState::Busy;
                already_read_some_lmao = true;

                // Create flags and add every direct and group flags you've just seen
                let mut flags = Flags::new();
                last_flags.into_iter().for_each(|flag_name| {
                    flags.add_direct_flag(flag_name);
                });
                group_flags.into_iter().for_each(|flag_name| {
                    flags.add_group_flag(flag_name);
                });

                let mut file = FileTree::new_regular_with_extra(value, Some(flags));

                // reinit for next iterations
                last_flags = vec![];
                group_flags = vec![];

                if let Some((LexToken::SymlinkArrow, _)) = tokens_iter.peek() {
                    if let Some((LexToken::Value(target), _)) = tokens_iter.nth(1) {
                        file.as_symlink(target);
                    } else {
                        return Err(ParserError::new(
                            current_line,
                            current_column,
                            ParserErrorKind::MissingSymlinkTarget,
                        ));
                    }
                }
                file_stack.push(file);
            },

            LexToken::DoubleDots => {
                // optional
            },

            LexToken::OpenBracket => {
                brackets_open_position.push((current_line, current_column));
                // Removed, need to study this again
                // assert!(matches!(read_state, ParserState::Busy), "WIP parser");
                read_state = ParserState::Clear;
                // If trying to open nothing, fail
                if !already_read_some_lmao {
                    return Err(ParserError::new(
                        current_line,
                        current_column,
                        ParserErrorKind::BracketUnexpectedOpen,
                    ));
                }

                assert!(!file_stack.is_empty());

                depth += 1;
                file_stack.last_mut().unwrap().as_directory(vec![]);
                quantity_stack.push(0);
                already_read_some_lmao = false;
            },

            LexToken::CloseBracket => {
                brackets_open_position.pop();

                if depth == 0 {
                    return Err(ParserError::new(
                        current_line,
                        current_column,
                        ParserErrorKind::BracketUnexpectedClose,
                    ));
                }
                already_read_some_lmao = true;
                depth -= 1;
                let mut vec: Vec<FileTree> = vec![];

                let quantity_in_group = quantity_stack.pop().unwrap();
                for _ in 0..quantity_in_group {
                    vec.push(file_stack.pop().unwrap());
                }
                // Reversed
                let vec = vec.into_iter().rev().collect();

                file_stack.last_mut().expect("should").as_directory(vec);
            },

            LexToken::Separator(separator) => {
                if depth == 0 && *separator == ',' {
                    return Err(ParserError::new(
                        current_line,
                        current_column,
                        ParserErrorKind::CommasOutsideOfBrackets,
                    ));
                }
                read_state = ParserState::Clear;

                if *separator == '\n' {
                    current_line += 1;
                    current_line_start_index = range.start;
                }
            },

            LexToken::Group(group) => {
                // This is intentionally mad bad
                groups_seen.entry(group.to_string()).or_insert_with(|| {
                    group_order.push(group.clone());
                });

                // Add everything from PREVIOUS group
                update_map_group(&mut map, current_group, &mut file_stack);
                // Update the group for the next entries
                current_group = group.into();

                // The last flags you've seen, are actually group_flags
                group_flags = last_flags;
                last_flags = vec![]; // reinit

                // After a group, we expect a line break
                match tokens_iter.peek() {
                    None | Some((LexToken::Separator('\n'), ..)) => {},
                    // TODO: show debug information
                    _other => panic!("We expected line break after this group"),
                }
            },

            // doing this
            LexToken::Flags(flags) => {
                // Flags not clear yet to read more flags
                if !last_flags.is_empty() {
                    return Err(ParserError::new(
                        current_line,
                        current_column,
                        ParserErrorKind::FlagAfterFlag,
                    ));
                }
                last_flags = flags.clone();
            },

            // João Marcos!! Logos!! editar isso pls
            LexToken::SymlinkArrow => {
                unreachable!("Unexpected SymlinkArrow!");
            },

            LexToken::LexError => {
                eprintln!("LexError => '{}'", &original_text[range]);
            },
        }
    }

    if depth != 0 {
        // Only show the inner bracket problem for now, even if there are multiple
        // unclosed
        let (start, end) = brackets_open_position.last().expect("should bro");
        return Err(ParserError::new(*start, *end, ParserErrorKind::BracketUnclosed));
    }

    update_map_group(&mut map, current_group, &mut file_stack);
    fn propagate_to_children(ft: &mut FileTree, accumulated_path: &mut PathBuf) {
        let old_current: PathBuf = ft.path().clone();
        *ft.path_mut() = accumulated_path.join(ft.path());
        accumulated_path.push(old_current);
        if let Some(children) = ft.children_mut() {
            children.iter_mut().for_each(|x| propagate_to_children(x, accumulated_path));
        }
        accumulated_path.pop();
    }

    for ft in map.values_mut().flat_map(|value| value.iter_mut()) {
        propagate_to_children(ft, &mut PathBuf::new());
    }

    Ok((map, group_order))
}

impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "moao tree: ")?;
        if let ParserErrorKind::BracketUnclosed = self.kind {
            // "close those brackets man!!!",
            write!(f, "bracket at {}:{} is unclosed!", self.line, self.column)?;
        } else {
            write!(f, "moao tree: at {}:{}: ", self.line, self.column)?;
        }

        use ParserErrorKind::*;
        match self.kind {
            BracketUnclosed => Ok(()),
            BracketUnexpectedClose => {
                write!(f, "unexpected close brackets, what are you closing?")
            },
            BracketUnexpectedOpen => {
                write!(f, "what are you trying to open there?????")
            },
            CommasOutsideOfBrackets => {
                write!(f, "no commas alowed outsite of scopes")
            },
            MissingSymlinkTarget => {
                write!(f, "arrow without the plim plimplimplim")
            },
            FlagAfterFlag => {
                write!(f, "Flag after flag problemo")
            },
        }
    }
}
