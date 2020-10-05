use crate::{error::*, file::File};

use std::{
    fs,
    path::{Path, PathBuf},
};

/// Fill a Vec with our own File struct
pub fn collect_directory_chidren(
    path: impl AsRef<Path>,
    follow_symlinks: bool,
) -> Result<Vec<File>> {
    let path = path.as_ref().to_path_buf();
    // if !FileType::from_path_shallow(&path, follow_symlinks)?.is_dir() {
    //     return Err(DotaoError::NotADirectory);
    // }

    let dirs = fs::read_dir(&path)/*.map_err(|source| DotaoError::ReadError {
        path: path.clone(),
        source,
    })*/?;

    let mut children = vec![];
    for entry in dirs {
        let entry = entry/*.map_err(|source| DotaoError::ReadError {
            path: path.clone(),
            source,
        })*/?;

        let file = File::from_path(&entry.path(), follow_symlinks)?;
        children.push(file);
    }
    Ok(children)
}

/// Follow symlink one level
pub fn symlink_target(path: impl AsRef<Path>) -> Result<PathBuf> {
    let path = path.as_ref();
    // if !path.exists() {
    //     return Err(DotaoError::NotFoundInFilesystem);
    // }

    let target = fs::read_link(&path)/*.map_err(|source| DotaoError::ReadError {
        path: path.to_path_buf(),
        source,
    })*/?;

    Ok(target)
}

/// Used by FileType `from_path*` function.
pub fn fs_filetype_from_path(path: impl AsRef<Path>, follow_symlink: bool) -> Result<fs::FileType> {
    let path = path.as_ref();
    // if !path.exists() {
    //     return Err(DotaoError::NotFoundInFilesystem);
    // }

    let metadata_function = if follow_symlink {
        fs::metadata
    } else {
        fs::symlink_metadata
    };

    let metadata = metadata_function(path)/*.map_err(|source| DotaoError::ReadError {
        path: path.to_path_buf(),
        source,
    })*/?;

    Ok(metadata.file_type())
}
