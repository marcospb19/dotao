// use crate::file_type::FileType;
use std::{error, fmt, io, path::PathBuf, result};

use file_tree::FileType;

/// Dotao's dotao::error::Result<T> = Result<T, DotaoError>
pub type Result<T> = result::Result<T, DotaoError>;

/// DotaoError covers all possible errors from this library
#[derive(Debug)]
pub enum DotaoError {
    LinkError { source_path: PathBuf, destination_path: PathBuf, source: io::Error },
    LinkError2 { source_path: PathBuf, destination_path: PathBuf, file_type: FileType<()> },
    ReadError { path: PathBuf, source: io::Error },
    NotFoundInFilesystem,
    NotADirectory,
    IoError(io::Error),
}

use DotaoError::*;

impl error::Error for DotaoError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            ReadError { source, .. } | LinkError { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Display ready for showing errors to users!
/// Error format: "err: more details: more details"
impl fmt::Display for DotaoError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ReadError { source, .. } => {
                write!(f, "Read error: ")?;
                source.fmt(f)
            },
            LinkError { source, source_path, destination_path } => {
                write!(
                    f,
                    "Link error: from '{}' to '{}': ",
                    source_path.display(),
                    destination_path.display()
                )?;
                source.fmt(f)
            },
            LinkError2 { source_path, destination_path, file_type } => write!(
                f,
                "Link error: failed to link {}, from '{}' to '{}'.",
                file_type,
                source_path.display(),
                destination_path.display()
            ),
            NotFoundInFilesystem => write!(f, "File not found"),
            NotADirectory => write!(f, "File is not a directory"),
            IoError(err) => err.fmt(f),
        }
    }
}

impl From<io::Error> for DotaoError {
    fn from(err: io::Error) -> Self {
        DotaoError::IoError(err)
    }
}
