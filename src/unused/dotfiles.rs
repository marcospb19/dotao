use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
};

use file_tree::{util::collect_directory_children, FileTree, FileType};

use crate::error::*;

#[derive(Debug, Default, Clone)]
pub struct DotfileGroup {
    pub starting_path: PathBuf,
    pub files: Vec<FileTree<()>>,
}

impl DotfileGroup {
    pub fn new(starting_path: PathBuf, files: Vec<FileTree<()>>) -> Self {
        DotfileGroup { starting_path, files }
    }

    pub fn from_directory_path(path: &impl AsRef<Path>, follow_symlinks: bool) -> Result<Self> {
        let path = path.as_ref().to_path_buf();

        if !path.exists() {
            return Err(DotaoError::NotFoundInFilesystem);
        } else if !FileType::<()>::from_path_shallow(&path, follow_symlinks).unwrap().is_dir() {
            return Err(DotaoError::NotADirectory);
        }

        // Recursively get all chidren from the directory path
        let files = collect_directory_children(&path, follow_symlinks).unwrap();
        if files.is_empty() {
            panic!("This is empty"); // Later treat this without panic
        }
        let mut group = DotfileGroup::new(path, files);

        // Adjust all path for file tree, in a way that, considering a file from it:
        // starting_path.join(file).exists()
        group.trim_starting_path_from_files();
        Ok(group)
    }

    pub fn trim_starting_path_from_files(&mut self) {
        // Calculate length of prefix to trim
        let len_to_trim = self.starting_path.iter().count();

        let mut stack: Vec<&mut FileTree<()>> = self.files.iter_mut().collect();

        // Pop elements and trim them, if they are directory, push each child, because
        // it also needs to be trimmed
        while let Some(file) = stack.pop() {
            // Trimming file.path
            file.path = file.path.iter().skip(len_to_trim).collect();

            // If it is a directory, push children
            if let FileType::Directory(children) = &mut file.file_type {
                stack.extend(children);
            }
        }
    }

    pub fn files_into_queue(&mut self) -> VecDeque<FileTree<()>> {
        let mut deque = VecDeque::new();

        while let Some(file) = self.files.pop() {
            if file.file_type.is_dir() {
                deque.push_back(file);
            } else {
                deque.push_front(file);
            }
        }

        deque
    }
}
