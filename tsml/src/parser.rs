use std::{collections::HashMap, path::PathBuf};

use crate::{
    lexer::SpannedLexToken, tags::Tags, FileTree, GroupsMap, LexToken, TsmlError, TsmlResult,
};

type Stack<T> = Vec<T>;

enum ParserState {
    Clear,
    Busy,
}

#[derive(Debug)]
pub enum ParserErrorKind {
    BracketUnclosed,
    BracketUnexpectedClose,
    BracketUnexpectedOpen,
    CommasOutsideOfBrackets,
    MissingSymlinkTarget,
    TagAfterTag,
}

fn update_map_group(map: &mut GroupsMap, group: String, files: &mut Stack<FileTree>) {
    let vec = map.entry(group).or_default();
    vec.append(files);
}

#[derive(Debug, Clone)]
pub struct TokenPosition {
    pub line: usize,
    pub column: usize,
    pub start_index: usize,
}

impl TokenPosition {
    fn new(line: usize, column: usize, start_index: usize) -> Self {
        Self { line, column, start_index }
    }
}

pub fn parse_tokens(
    spanned_tokens: Vec<SpannedLexToken>,
    original_text: &str,
) -> TsmlResult<(GroupsMap, Vec<String>)> {
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

    let mut group_tags = Vec::<String>::new();
    let mut last_tags = Vec::<String>::new();

    let mut group_order = vec!["main".to_string()];
    let mut groups_seen = HashMap::<String, ()>::new();
    groups_seen.insert("main".to_string(), ());

    while let Some((token, range)) = tokens_iter.next() {
        let current_column = range.start - current_line_start_index;
        let position = TokenPosition::new(current_line, current_column, current_line_start_index);

        match &token {
            LexToken::Value(value) => {
                *quantity_stack.last_mut().unwrap() += 1;

                if let ParserState::Busy = read_state {
                    panic!("busy when read this token: '{:?}'", token);
                    // panic!("{:?}", range);
                }
                read_state = ParserState::Busy;
                already_read_some_lmao = true;

                // Create tags and add every direct and group tags you've just seen
                let mut tags = Tags::new();
                last_tags.into_iter().for_each(|tag_name| {
                    tags.add_direct_tag(tag_name);
                });
                group_tags.into_iter().for_each(|tag_name| {
                    tags.add_group_tag(tag_name);
                });

                let mut file = FileTree::new_regular_with_extra(value, Some(tags));

                // reinit for next iterations
                last_tags = vec![];
                group_tags = vec![];

                if let Some((LexToken::SymlinkArrow, _)) = tokens_iter.peek() {
                    if let Some((LexToken::Value(target), _)) = tokens_iter.nth(1) {
                        file.to_symlink(target);
                    } else {
                        return Err(TsmlError::ParserError(
                            position,
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
                brackets_open_position.push(position.clone());
                // Removed, need to study this again
                // assert!(matches!(read_state, ParserState::Busy), "WIP parser");
                read_state = ParserState::Clear;
                // If trying to open nothing, fail
                if !already_read_some_lmao {
                    return Err(TsmlError::ParserError(
                        position,
                        ParserErrorKind::BracketUnexpectedOpen,
                    ));
                }

                assert!(!file_stack.is_empty());

                depth += 1;
                file_stack.last_mut().unwrap().to_directory(vec![]);
                quantity_stack.push(0);
                already_read_some_lmao = false;
            },

            LexToken::CloseBracket => {
                brackets_open_position.pop();

                if depth == 0 {
                    return Err(TsmlError::ParserError(
                        position,
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

                file_stack.last_mut().expect("should").to_directory(vec);
            },

            LexToken::Separator(separator) => {
                if depth == 0 && *separator == ',' {
                    return Err(TsmlError::ParserError(
                        position,
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

                // The last tags you've seen, are actually group_tags
                group_tags = last_tags;
                last_tags = vec![]; // reinit

                // After a group, we expect a line break
                match tokens_iter.peek() {
                    None | Some((LexToken::Separator('\n'), ..)) => {},
                    // TODO: show debug information
                    _other => panic!("We expected line break after this group"),
                }
            },

            // doing this
            LexToken::Tags(tags) => {
                // tags not clear yet to read more tags
                if !last_tags.is_empty() {
                    return Err(TsmlError::ParserError(position, ParserErrorKind::TagAfterTag));
                }
                last_tags = tags.clone();
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
        let position = brackets_open_position.last().expect("should bro");
        return Err(TsmlError::ParserError(position.clone(), ParserErrorKind::BracketUnclosed));
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
