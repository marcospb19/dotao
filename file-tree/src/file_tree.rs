//! `FileTree` implementation
//!
//! The functions with name suffixed by "symlink" do not follow symlinks at `path`, read the
//! documentation carefully.

use std::{
    env, fs, mem,
    path::{Path, PathBuf},
};

use file_type_enum::FileType as FileTypeEnum;

use crate::{
    error::*,
    iter::{FilesIter, PathsIter},
    util,
};

const PATH_DEFAULT: &str = "";

/// A recursive defined file tree that supports a generic field.
///
/// This enum has 3 variants, each of them have 2 named fields in common:
/// 1. `path: Pathbuf`, the relative path to the file.
/// 2. `extra: Option<T>`, a generic field that let's you customize the recursive structure.
///
/// - `FileTree::Directory` field `children` is a owned Vec of other child FileTrees.
/// - `FileTree::Symlink` field `target_path` is the pointed relative or absolute path.
///
/// Keep in mind that each `FileTree` inside a `FileTree::Directory` will have the path with all
/// parent components in it, so:
///
/// ```txt
/// // For this structure
///     "a": [
///         "b": [
///             "c"
///         ],
///     ]
/// // .path for each one is:
///     "a"
///     "a/b"
///     "a/c/c"
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FileTree<T> {
    // Normal file
    Regular { path: PathBuf, extra: Option<T> },
    // Directory, can contain other `FileTree`s inside
    Directory { path: PathBuf, extra: Option<T>, children: Vec<Self> },
    // Symbolic link, points to another location
    Symlink { path: PathBuf, extra: Option<T>, target_path: PathBuf },
}

impl<T> FileTree<T> {
    /// Creates a `FileTree::Regular` from arguments.
    pub fn new_regular(path: impl AsRef<Path>) -> Self {
        Self::new_regular_with_extra(path, Option::default())
    }

    /// Creates a `FileTree::Regular` with default arguments.
    pub fn regular_default() -> Self {
        Self::new_regular_with_extra(PATH_DEFAULT, Option::default())
    }

    /// Creates a `FileTree::Regular` with arguments, including `extra`.
    pub fn new_regular_with_extra(path: impl AsRef<Path>, extra: Option<T>) -> Self {
        let path = path.as_ref().to_path_buf();
        Self::Regular { path, extra }
    }

    /// Creates a `FileTree::Directory` from arguments.
    pub fn new_directory(path: impl AsRef<Path>, children: Vec<Self>) -> Self {
        Self::new_directory_with_extra(path, children, Option::default())
    }

    /// Creates a `FileTree::Directory` with default arguments.
    pub fn directory_default() -> Self {
        Self::new_directory_with_extra(PATH_DEFAULT, Vec::default(), Option::default())
    }

    /// Creates a `FileTree::Directory` with arguments, including `extra`.
    pub fn new_directory_with_extra(
        path: impl AsRef<Path>,
        children: Vec<Self>,
        extra: Option<T>,
    ) -> Self {
        let path = path.as_ref().to_path_buf();
        Self::Directory { path, children, extra }
    }

    /// Creates a `FileTree::Symlink` from arguments.
    pub fn new_symlink(path: impl AsRef<Path>, target_path: impl AsRef<Path>) -> Self {
        Self::new_symlink_with_extra(path, target_path, Option::default())
    }

    /// Creates a `FileTree::Symlink` with default arguments.
    pub fn symlink_default() -> Self {
        Self::new_symlink_with_extra(PATH_DEFAULT, PATH_DEFAULT, Option::default())
    }

    /// Creates a `FileTree::Symlink` with arguments, including `extra`.
    pub fn new_symlink_with_extra(
        path: impl AsRef<Path>,
        target_path: impl AsRef<Path>,
        extra: Option<T>,
    ) -> Self {
        let path = path.as_ref().to_path_buf();
        let target_path = target_path.as_ref().to_path_buf();
        Self::Symlink { path, target_path, extra }
    }

    // Private implementation
    fn __collect_from_directory(path: &Path, follow_symlinks: bool) -> FtResult<Vec<Self>> {
        if !path.exists() {
            return Err(FtError::NotFoundError(path.to_path_buf()));
        } else if !FileTypeEnum::from_path(path)?.is_directory() {
            return Err(FtError::NotADirectoryError(path.to_path_buf()));
        }
        let dirs = fs::read_dir(path)?;

        let mut children = vec![];
        for entry in dirs {
            let entry = entry?;
            let file = Self::__from_path(&entry.path(), follow_symlinks)?;
            children.push(file);
        }
        Ok(children)
    }

    /// Collects a `Vec` of `FileTree` from `path` that is a directory.
    pub fn collect_from_directory(path: impl AsRef<Path>) -> FtResult<Vec<Self>> {
        Self::__collect_from_directory(path.as_ref(), true)
    }

    /// Collects a `Vec` of `FileTree` from `path` that is a directory, entries can be symlinks.
    pub fn collect_from_directory_symlink(path: impl AsRef<Path>) -> FtResult<Vec<Self>> {
        Self::__collect_from_directory(path.as_ref(), false)
    }

    // Private implementation
    fn __collect_from_directory_cd(path: &Path, follow_symlinks: bool) -> FtResult<Vec<Self>> {
        let previous_path = env::current_dir()?;
        debug_assert!(path.is_absolute());
        env::set_current_dir(path)?;
        let result = Self::__collect_from_directory(&Path::new("."), follow_symlinks);
        env::set_current_dir(previous_path)?;
        result
    }

    /// Collects a `Vec` of `FileTree` from `path` that is a directory.
    pub fn collect_from_directory_cd(path: impl AsRef<Path>) -> FtResult<Vec<Self>> {
        Self::__collect_from_directory_cd(path.as_ref(), false)
    }

    /// Collects a `Vec` of `FileTree` from `path` that is a directory, entries can be symlinks.
    pub fn collect_from_directory_symlink_cd(path: impl AsRef<Path>) -> FtResult<Vec<Self>> {
        Self::__collect_from_directory_cd(path.as_ref(), false)
    }

    // Internal implementation of `from_path` and `from_path_symlink`
    fn __from_path(path: &Path, follow_symlinks: bool) -> FtResult<Self> {
        let get_file_type =
            if follow_symlinks { FileTypeEnum::from_path } else { FileTypeEnum::from_symlink_path };

        match get_file_type(path)? {
            FileTypeEnum::Regular => Ok(Self::new_regular(path)),
            FileTypeEnum::Directory => {
                let children = Self::__collect_from_directory(path, follow_symlinks)?;
                Ok(Self::new_directory(path, children))
            },
            FileTypeEnum::Symlink => {
                let target_path = util::symlink_target(path)?;
                Ok(Self::new_symlink(path, target_path))
            },
            _ => Err(FtError::UnexpectedFileTypeError(path.to_path_buf())),
        }
    }

    /// Builds a `FileTree` from `path`, follows symlinks.
    ///
    /// Similar to `from_path_symlink`.
    ///
    /// If file at `path` is a regular file, will return a `FileTree::Regular`.
    /// If file at `path` is a directory file, `FileTree::Directory` (with .children).
    ///
    /// # Errors:
    /// - If `Io::Error` from `fs::metadata(path)`
    /// - If it is a directory, and `Io::Error` from `fs::read_dir(path)` iterator usage
    /// - If [unexpected file type] at `path`
    ///
    /// This function traverses symlinks until final destination, and then reads it, so it can never
    /// return `Ok(FileTree::Symlink { .. ]})`, if you wish otherwise, use
    /// `FileTree::from_path_symlink` instead.
    ///
    /// [unexpected file type]: docs.rs/file_type_enum
    pub fn from_path(path: impl AsRef<Path>) -> FtResult<Self> {
        Self::__from_path(path.as_ref(), true)
    }

    /// Builds a `FileTree` from `path`, follows symlinks.
    ///
    /// Similar to `from_path_symlink`.
    ///
    /// If file at `path` is a regular file, will return a `FileTree::Regular`.
    /// If file at `path` is a directory file, `FileTree::Directory` (with `children` field).
    /// If file at `path` is a symlink file, `FileTree::Symlink` (with `target_path` field).
    ///
    /// # Errors:
    /// - If `Io::Error` from `fs::metadata(path)`
    /// - If it is a directory, and `Io::Error` from `fs::read_dir(path)` iterator usage
    /// - If it is a symlink, and `Io::Error` from `fs::read_link(path)`
    /// - If [unexpected file type] at `path`
    ///
    /// If you wish to traverse symlinks until final destination, instead, use
    /// `FileTree::from_path`.
    ///
    /// [unexpected file type]: docs.rs/file_type_enum
    pub fn from_path_symlink(path: impl AsRef<Path>) -> FtResult<Self> {
        Self::__from_path(path.as_ref(), false)
    }

    // Internal
    fn ___from_path_cd(path: &Path, follow_symlinks: bool) -> FtResult<Self> {
        let previous_path = env::current_dir()?;
        debug_assert!(path.is_absolute());
        env::set_current_dir(path)?;
        let result = Self::__from_path(&Path::new("."), follow_symlinks);
        env::set_current_dir(previous_path)?;
        result
    }

    /// `cd` into path, run `from_path`, and come back.
    ///
    /// TODO explain here why this is useful
    pub fn from_path_cd(path: impl AsRef<Path>) -> FtResult<Self> {
        Self::___from_path_cd(path.as_ref(), true)
    }

    /// `cd` into path, run `from_path_symlink`, and come back.
    ///
    /// TODO explain here why this is useful
    pub fn from_cd_symlink_path(path: impl AsRef<Path>) -> FtResult<Self> {
        Self::___from_path_cd(path.as_ref(), false)
    }

    /// Creates a `FileTree` from path text.
    ///
    /// For example: `FileTree::from_path_text("a/b/c")`, results in the following structure:
    /// ```txt
    /// "a": [
    ///   "b": [
    ///     "c"
    ///   ]
    /// ]
    /// ```
    ///
    /// Examples:
    /// TODO make here a example where you create that and assert_eq the paths returned by the iters
    /// lmao
    // Ok so the implementation is a weird post-order recursion, need more documentation
    // explaining!!!!!
    //
    // Could also be optimized using another function
    pub fn from_path_text(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref();
        let size = path.iter().count();
        Self::__other(&mut path.iter(), size)
    }

    fn __other(iter: &mut std::path::Iter, how_many: usize) -> Self {
        if true {
            todo!("test if this is working please")
        } else if how_many <= 1 {
            // Only one component, only one file
            FileTree::new_regular(iter.as_path())
        } else {
            // Multiple components, create a directory with the leftmost one and add the rest of
            // them as nested children
            let child = Self::__other(iter, how_many - 1);
            // Remove the last one
            iter.next_back();

            // Create current and return it
            let path = iter.as_path();
            FileTree::new_directory(path, vec![child])
        }
    }

    /// Fix paths for the macro
    ///
    /// needs docs
    pub fn fix(&mut self) {
        let parent_path_copy = self.path().clone();
        if let Some(children) = self.children_mut() {
            for child in children.iter_mut() {
                *child.path_mut() = parent_path_copy.join(child.path());
                if let Some(target) = child.target_mut() {
                    *target = parent_path_copy.join(&target);
                }
                child.fix();
            }
        }
    }

    /// Reference to children vec if self.is_directory().
    pub fn children(&self) -> Option<&Vec<Self>> {
        match self {
            FileTree::Directory { children, .. } => Some(children),
            _ => None,
        }
    }

    /// Reference to children vec if self.is_directory(), mutable.
    pub fn children_mut(&mut self) -> Option<&mut Vec<Self>> {
        match self {
            FileTree::Directory { children, .. } => Some(children),
            _ => None,
        }
    }

    /// Reference to target_path if self.is_symlink().
    pub fn target(&self) -> Option<&PathBuf> {
        match self {
            FileTree::Symlink { target_path, .. } => Some(target_path),
            _ => None,
        }
    }

    /// Reference to target_path if self.is_symlink(), mutable.
    pub fn target_mut(&mut self) -> Option<&mut PathBuf> {
        match self {
            FileTree::Symlink { target_path, .. } => Some(target_path),
            _ => None,
        }
    }

    /// Apply a closure for each direct child of this FileTree.
    ///
    /// Only 1 level deep.
    pub fn apply_to_children0(&mut self, f: impl FnMut(&mut Self)) {
        if let Some(children) = self.children_mut() {
            children.iter_mut().for_each(f);
        }
    }

    /// Apply a closure to all direct and indirect descendants inside of this structure.
    ///
    /// Calls recursively for all levels.
    pub fn apply_to_all_children1(&mut self, f: impl FnMut(&mut Self) + Copy) {
        if let Some(children) = self.children_mut() {
            children.iter_mut().for_each(|x| x.apply_to_all_children1(f));
            children.iter_mut().for_each(f);
        }
    }

    /// Apply a closure to all direct and indirect descendants inside, also includes root.
    ///
    /// Calls recursively for all levels.
    pub fn apply_to_all(&mut self, mut f: impl FnMut(&mut Self) + Copy) {
        f(self);
        if let Some(children) = self.children_mut() {
            for child in children.iter_mut() {
                child.apply_to_all(f);
            }
        }
    }

    pub fn path(&self) -> &PathBuf {
        match self {
            Self::Regular { path, .. }
            | Self::Directory { path, .. }
            | Self::Symlink { path, .. } => path,
        }
    }

    pub fn path_mut(&mut self) -> &mut PathBuf {
        match self {
            Self::Regular { path, .. }
            | Self::Directory { path, .. }
            | Self::Symlink { path, .. } => path,
        }
    }

    pub fn extra(&self) -> &Option<T> {
        match self {
            Self::Regular { extra, .. }
            | Self::Directory { extra, .. }
            | Self::Symlink { extra, .. } => extra,
        }
    }

    pub fn extra_mut(&mut self) -> &mut Option<T> {
        match self {
            Self::Regular { extra, .. }
            | Self::Directory { extra, .. }
            | Self::Symlink { extra, .. } => extra,
        }
    }

    /// Iterator of all `FileTree`s in the structure
    pub fn files(&self) -> FilesIter<T> {
        FilesIter::new(self)
    }

    /// Shorthand for `self.files().paths()`, see link to [`.paths()`] method
    ///
    /// [`.paths()`]: super::iter::FilesIter::paths
    pub fn paths(&self) -> PathsIter<T> {
        self.files().paths()
    }

    /// Shorthand for `file.file_type.is_regular()`
    pub fn is_regular(&self) -> bool {
        matches!(self, Self::Regular { .. })
    }

    /// Shorthand for `file.file_type.is_dir()`
    pub fn is_dir(&self) -> bool {
        matches!(self, Self::Directory { .. })
    }

    /// Shorthand for `file.file_type.is_symlink()`
    pub fn is_symlink(&self) -> bool {
        matches!(self, Self::Symlink { .. })
    }

    pub fn as_regular(&mut self) {
        match self {
            Self::Regular { .. } => {},
            Self::Directory { path, extra, .. } | Self::Symlink { path, extra, .. } => {
                let path = mem::take(path);
                let extra = mem::take(extra);
                *self = Self::Regular { path, extra };
            },
        }
    }

    pub fn as_directory(&mut self, children: Vec<Self>) {
        match self {
            Self::Regular { path, extra }
            | Self::Directory { path, extra, .. }
            | Self::Symlink { path, extra, .. } => {
                let path = mem::take(path);
                let extra = mem::take(extra);
                *self = Self::Directory { path, children, extra };
            },
        }
    }

    pub fn as_symlink(&mut self, target_path: impl AsRef<Path>) {
        match self {
            Self::Regular { path, extra }
            | Self::Directory { path, extra, .. }
            | Self::Symlink { path, extra, .. } => {
                let path = mem::take(path);
                let extra = mem::take(extra);
                let target_path = target_path.as_ref().to_path_buf();
                *self = Self::Symlink { path, target_path, extra };
            },
        }
    }
}
