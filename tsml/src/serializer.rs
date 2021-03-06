use crate::{FileTree, Groups, Tags, TsmlError, TsmlResult};

// TODO: preserve comments, somehow
// TODO: fix order
// TODO: preserve order of values (iter.rs)

// Where T is whatever it wants to be
pub fn groups_to_tsml(groups: &Groups) -> TsmlResult<String> {
    let mut text = String::new();
    for (i, group) in groups.map.iter().enumerate() {
        // Group separator
        if i != 0 {
            text.push('\n');
        }
        let (key, files) = group;
        add_group_to_tsml(&mut text, key, files)?;
    }
    Ok(text)
}

fn indent(text: &mut String, levels: usize) {
    for _ in 0..levels {
        text.push_str("    ");
    }
}

fn add_tags(text: &mut String, tags: &Option<Tags>) {
    if let Some(tags) = tags {
        if !tags.direct_tags.is_empty() {
            // TODO: preserve order of the tags
            text.push_str(format!("({}", tags.direct_tags.iter().next().unwrap()).as_str());
            for tag in tags.direct_tags.iter().skip(1) {
                text.push_str(format!(", {}", tag).as_str());
            }
            text.push_str(") ");
        }
    } // end of adding tags
}

fn close_bracket(text: &mut String, at_indent_level: usize) {
    indent(text, at_indent_level);
    text.push_str("]\n");
}

// Todo: think about what to do here to deal with group tags
fn add_group_to_tsml(text: &mut String, key: &str, files: &[FileTree]) -> TsmlResult<()> {
    if key != "main" {
        text.push_str(format!("- [{}]\n", key.trim_end_matches('/')).as_str());
    }

    let mut last_depth = 0;
    for file_tree in files.iter() {
        // For each file inside of this bad boi
        let mut file_iter = file_tree.files();
        while let Some(file) = file_iter.next() {
            // If left a directory, close brackets
            for level in (file_iter.depth()..last_depth).rev() {
                close_bracket(text, level);
            }

            // Indent
            indent(text, file_iter.depth());
            // Add tags
            add_tags(text, file.extra());
            // Write file name
            // TODO fix string_lossy

            let file: &FileTree = file;

            text.push_str(
                format!("\"{}\"", file.path().file_name().expect("unexpected").to_str().expect(""))
                    .as_str(),
            );
            // Draw regular file
            match file {
                FileTree::Regular { .. } => {},
                FileTree::Directory { .. } => {
                    text.push_str(": [");
                },
                FileTree::Symlink { target_path, .. } => {
                    let target_path = target_path.file_name().ok_or(TsmlError::PathWithoutName)?;
                    let target_path = target_path
                        .to_str()
                        .ok_or_else(|| TsmlError::NonUtf8Path(target_path.into()))?;
                    text.push_str(&format!(" > \"{}\"", target_path));
                },
            }
            text.push('\n');
            last_depth = file_iter.depth();
        }
    }

    // If there's any depth left, close all brackets
    for level in (0..last_depth).rev() {
        close_bracket(text, level);
    }
    Ok(())
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::tags;

//     #[test]
//     fn asd() {
//         let file = File::new("asd", FileType::<tags>::Regular);
//         for a in file.files() {
//             println!("{:?}", a);
//         }
//         panic!();
//     }
// }
