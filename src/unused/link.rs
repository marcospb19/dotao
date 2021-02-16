use std::{
    collections::VecDeque,
    fs, io,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
};

use file_tree::{FileTree, FileType};

use crate::{dotfiles::DotfileGroup, error::*};

#[derive(Debug, Default, Clone)]
pub struct LinkBehavior {
    pub interactive_mode: bool,
    pub overwrite_files: bool,
    pub overwrite_directories: bool,
    pub overwrite_symbolic_links: bool,
}

impl LinkBehavior {
    pub fn new(
        interactive_mode: bool,
        overwrite_files: bool,
        overwrite_directories: bool,
        overwrite_symbolic_links: bool,
    ) -> Self {
        LinkBehavior {
            interactive_mode,
            overwrite_files,
            overwrite_directories,
            overwrite_symbolic_links,
        }
    }
}

#[derive(Default, Debug)]
pub struct LinkInformation {
    pub groups: Vec<DotfileGroup>,
    pub payload: LinkPayload,
    pub link_behavior: LinkBehavior,
    pub errors: Vec<Box<dyn std::error::Error + 'static>>,
    pub conflicts: Vec<Box<dyn std::error::Error + 'static>>,
}

#[derive(Default, Clone, Debug)]
pub struct LinkPayload {
    links: Vec<(PathBuf, PathBuf)>,
    deletes: Vec<(PathBuf, FileType<()>)>,
}

impl LinkInformation {
    pub fn new() -> Self {
        LinkInformation {
            errors: vec![],
            groups: vec![],
            payload: LinkPayload::default(),
            link_behavior: LinkBehavior::default(),
            conflicts: vec![],
        }
    }

    pub fn prepare_linkage_to_home(&mut self, home_path: &Path) -> Result<()> {
        let copy = self.groups.clone();
        for group in copy.into_iter() {
            //
            let mut deque: VecDeque<FileTree<()>> = VecDeque::from(group.files.clone());
            // println!("{:#?}", deque);

            while let Some(file) = deque.pop_front() {
                // Debug
                // println!("{:#?}", file.path);
                // println!("{:#?}", file);

                // We have source and destination

                // Symlink source_path needs to be absolute
                let source_path = group.starting_path.join(&file.path).canonicalize()?;
                if !source_path.exists() {
                    panic!("This should exist");
                }

                let destination_path = home_path.join(&file.path);
                // No file there, just link it
                if !destination_path.exists() {
                    self.payload.links.push((source_path.clone(), destination_path.clone()));
                    continue;
                }

                let source_file_type = file.file_type;
                if let FileType::Symlink { .. } = source_file_type {
                    panic!("No symlinks allowed in source");
                }

                // THIS IS SHALLOW!
                let destination_file_type =
                    FileType::from_path_shallow(&destination_path, false).unwrap();
                match destination_file_type {
                    FileType::Regular => {
                        self.link_check_for_regular_file(
                            source_path,
                            destination_path,
                            source_file_type,
                            destination_file_type,
                        )
                        .unwrap();
                    },
                    FileType::Directory { .. } => {
                        match source_file_type {
                            FileType::Regular => panic!("Cannot delete directory to link file."),
                            FileType::Directory(children) => {
                                for child in children.iter() {
                                    deque.push_back(child.clone());
                                }
                                //
                                //
                            },
                            FileType::Symlink { .. } => unreachable!(),
                        }
                        // let should_add_children =
                        //     self.link_check_for_directory(source_path,
                        // destination_path);
                    },
                    FileType::Symlink(target_path) => {
                        self.link_check_for_symlink(
                            source_path,
                            destination_path,
                            source_file_type,
                            &target_path,
                        )
                        .unwrap();
                    },
                }
            }
        }
        Ok(())
    }

    fn link_check_for_regular_file(
        &mut self,
        source_path: PathBuf,
        destination_path: PathBuf,
        _source_file_type: FileType<()>,
        destination_file_type: FileType<()>,
    ) -> io::Result<()> {
        if self.link_behavior.overwrite_files {
            println!("Deleting file at '{}'?", destination_path.display());
            self.payload.deletes.push((destination_path.clone(), destination_file_type));
            self.payload.links.push((source_path, destination_path));
        } else {
            println!("Error deleting file at '{}'?", destination_path.display());
            self.conflicts.push(Box::new(DotaoError::LinkError2 {
                file_type: FileType::from_path_shallow(&destination_path, false).unwrap(),
                source_path,
                destination_path,
            }))
        }
        Ok(())
    }

    fn link_check_for_symlink(
        &mut self,
        source_path: PathBuf,
        destination_path: PathBuf,
        destination_file_type: FileType<()>,
        current_path: &PathBuf,
    ) -> io::Result<()> {
        if *current_path == source_path {
            return Ok(()); // Nothing to do, already linked
        } else if self.link_behavior.overwrite_symbolic_links {
            self.payload.deletes.push((destination_path.clone(), destination_file_type));
            self.payload.links.push((source_path, destination_path));
        } else {
            panic!("Cannot overwrite symlink");
        }

        Ok(())
    }

    pub fn configure_behavior(&mut self, link_behavior: LinkBehavior) {
        self.link_behavior = link_behavior;
    }

    pub fn add_group(&mut self, group: DotfileGroup) {
        self.groups.push(group);
    }

    pub fn critical_error_occurred(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn show_errors(&self) {
        eprintln!("List of errors:");
        for error in &self.errors {
            eprintln!("{:#?}", error);
        }
    }

    pub fn proceed_and_link(&self) -> Result<()> {
        for (path, file_type) in &self.payload.deletes {
            match file_type {
                FileType::Regular => fs::remove_file(path)?,
                _ => unimplemented!(),
            }
        }

        for (source, dest) in &self.payload.links {
            symlink_with_checks(source, dest)?;
        }

        Ok(())
    }
}

/// Wrap std::os::unix::fs::symlink with Dotao's Result<()>, extra checks
pub fn symlink_with_checks(src: &impl AsRef<Path>, dest: &impl AsRef<Path>) -> Result<()> {
    let (src, dest) = (src.as_ref(), dest.as_ref());
    if !src.exists() {
        return Err(DotaoError::NotFoundInFilesystem);
    } else if true {
        // Check if dest.exists()!!!!, overwrite???? vixe!
        // Check permissions?
    }

    symlink(src, dest).map_err(|source| DotaoError::LinkError {
        source_path: src.to_path_buf(),
        destination_path: dest.to_path_buf(),
        source,
    })
}
