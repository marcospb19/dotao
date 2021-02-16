use std::{
    env,
    ffi::OsStr,
    fs,
    io::{BufWriter, Write},
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    process,
};

use indoc::indoc;
use lazy_static::lazy_static;

use super::{cli, error};

lazy_static! {
    static ref CURRENT_DIR: PathBuf = env::current_dir()
        .unwrap_or_else(|err| error!("Failed to read curent directory path: '{}'.", err));
}

fn bytes_to_uft(asd: impl AsRef<OsStr>) -> String {
    let text = format!("{:?}", asd.as_ref());
    text.trim_matches('"').to_string()
}

fn run_status_command() {
    println!("run status command");
}

fn run_init_command(force_flag: bool) {
    fn is_currently_in_git_repository() -> bool {
        let current_dir = &CURRENT_DIR;
        let mut path: &Path = &current_dir;
        loop {
            if path.join(".git").exists() {
                return true;
            } else if let Some(parent) = path.parent() {
                path = parent;
            } else {
                return false;
            }
        }
    }

    fn is_in_dotfiles_folder() -> bool {
        let current_dir = &CURRENT_DIR;
        match current_dir.file_name() {
            None => false,
            Some(file_name) => file_name == Path::new("dotfiles"),
        }
    }

    // Checks
    if !is_currently_in_git_repository() && !force_flag {
        error!(
            "You are not inside a git repository, we recommend you to first run `git init`.\n\
             To ignore this recommendation, run `dotao init --force` instead."
        );
    } else if !is_in_dotfiles_folder() && !force_flag {
        error!(
            "You are not inside the '~/dotfiles' folder, we recomend creating it and running `dotao` in it.\n\
             To ignore this recommendation, run `dotao init --force` instead."
        );
    } else if Path::new("dotao.tsml").exists() {
        error!(
            "You ran `dotao init`, but the 'dotao.tsml' file already exists!\n\
             Delete the file manually if you with to restart the tree configuration.\n\
             This action may not be reversible."
        );
    }

    let mut new_dotao_tsml = fs::File::create("dotao.tsml")
        .unwrap_or_else(|err| error!("Error while trying to create file 'dotao.tsml': {}.", err));

    write!(
        new_dotao_tsml,
        indoc!(
            "//       __     __              __
             //   ___/ /__  / /____ ____    / /________ ___
             //  / _  / _ \\/ __/ _ `/ _ \\  / __/ __/ -_) -_)
             //  \\___/\\___/\\__/\\___/\\___/  \\__/_/  \\__/\\__/
             //
             // Tree configuration file, see more at
             // https://github.com/marcospb19/dotao (TODO, there's no info there)
             //
             // Tips: Some commands you can type, and what they do:
             //   - `dotao add <folders>`, to add a group tree to this file.
             //   - `dotao status`, to see what's going on.
             //   - `dotao link`, to link added groups your home directory.
             //
             //
             // (Only the comments in this header block are persistent)
            "
        )
    )
    .unwrap_or_else(|err| error!("Error while trying to write to 'dotao.tsml': {}.", err));

    // Success!
    println!(
        "Tree file successfully created at '{}'.",
        bytes_to_uft(CURRENT_DIR.join("dotao.tsml"))
    );
    println!(
        "For help, type `dotao --help`.\n\
         See also the (TODO) full tutorial at https://github.com/marcospb19/dotao ."
    );
}

fn run_add_command(group_names: &[&str]) {
    let content = fs::read_to_string("dotao.tsml").unwrap();
    let mut tree = tsml::Groups::from_text(&content);
    // The header of the file is made of the starting comments and blank lines
    let mut header = content
        .lines()
        .take_while(|line| line.starts_with("//") || line.is_empty())
        .collect::<Vec<&str>>();

    let amount_of_trailing_empty = header.iter().rev().take_while(|line| line.is_empty()).count();
    // Remove excessive empty lines
    for _ in 0..amount_of_trailing_empty {
        header.pop();
    }

    let group_files: Vec<Vec<tsml::FileTree>> = group_names
        .iter()
        .map(|path| {
            tsml::FileTree::collect_from_directory(path).unwrap_or_else(|err| {
                error!("Error while trying to read `add` arguments: {:?}.", err)
            })
        })
        .collect();

    // Updating groups
    for (name, vec_of_files) in group_names.iter().zip(group_files) {
        *tree.map.entry(name.to_string()).or_default() = vec_of_files;
    }

    // Override file
    let file = fs::File::create("dotao.tsml").unwrap_or_else(|err| {
        error!("Unable to open dotao.tsml to edit (write) it: {}.", err);
    });

    let mut writer = BufWriter::new(file);
    for comment in header.iter() {
        writeln!(writer, "{}", comment).unwrap_or_else(|err| error!("Unable to write! {}", err));
    }
    let tree_content = tsml::groups_to_tsml(&tree);
    write!(writer, "{}", tree_content).unwrap_or_else(|err| error!("Unable to write! {}", err));
}

fn run_link_command() {
    let home = env::var_os("HOME").expect("No home detected.");
    let tree = tsml::Groups::from_path("dotao.tsml").unwrap();

    let tsml_names: Vec<_> = tree.map.keys().collect();
    let tsml_trees: Vec<_> = tree.map.values().collect();

    for (name, tree_vec) in tsml_names.iter().zip(tsml_trees) {
        for tree in tree_vec.iter() {
            for file in tree.files() {
                let semi_path = file.path();
                let dotfiles_path = Path::new(name).join(&semi_path);
                let dotfiles_path = dotfiles_path.canonicalize().unwrap_or_else(|err| {
                    error!("Can't canonicalize to '{}': {}.", dotfiles_path.display(), err)
                });
                let home_path = Path::new(&home).join(&semi_path);
                // let home_path = home_path.canonicalize().unwrap_or_else(|err| {
                //     error!("Can't canonicalize to '{}': {}.", home_path.display(), err)
                // });

                // Let's naively skip what we've already linked
                if home_path.exists() {
                    continue;
                }
                match file {
                    tsml::FileTree::Regular { .. } => {
                        symlink(&dotfiles_path, &home_path).unwrap_or_else(|err| {
                            error!(
                                "Error while trying to make link for regular file '{}' -> '{}': {}",
                                dotfiles_path.display(),
                                home_path.display(),
                                err
                            )
                        });
                    },
                    tsml::FileTree::Directory { children, .. } => {
                        // Verify if should link or create directory
                        // If there's no children, link, else, create directory
                        if children.is_empty() {
                            symlink(&dotfiles_path, &home_path).unwrap_or_else(|err| {
                                error!(
                                "Error while trying to make link for directory '{}' -> '{}': {}",
                                dotfiles_path.display(),
                                home_path.display(),
                                err
                            );
                            });
                        } else {
                            fs::create_dir_all(&home_path)
                                .expect("Error while trying to create directory.");
                        }
                    },
                    tsml::FileTree::Symlink { .. } => todo!(),
                }
            }
        }
    }
}

fn run_unlink_command() {
    todo!();
}

fn run_remove_command() {
    println!("run remove command");
}

pub fn run() {
    if env::args().len() == 1 {
        run_status_command();
    }
    let args = cli::parse_args();

    match args.subcommand() {
        ("status", Some(_)) => {
            run_status_command();
        },
        ("init", Some(init_matches)) => {
            // Flag
            let force = init_matches.is_present("force");
            run_init_command(force);
        },
        ("add", Some(add_matches)) => {
            let groups = add_matches.values_of("groups").unwrap(); // Safe
            let groups: Vec<&str> = groups.collect();
            run_add_command(&groups);
        },
        ("link", Some(_)) => {
            run_link_command();
        },
        ("unlink", Some(_)) => {
            run_unlink_command();
        },
        ("rm", Some(_)) => {
            run_remove_command();
        },
        _ => unreachable!(),
    }

    // let mut groups: Vec<DotfileGroup> = vec![];
    // let mut error_occurred = false;

    // // For each arg of GROUPS
    // for group_path in args.values_of("GROUPS").unwrap() {
    //     // Try to transform into DotfileGroup
    //     // Symlinks in dotfiles work, so follow them
    //     let group: Result<DotfileGroup> =
    // DotfileGroup::from_directory_path(&group_path, true);

    //     if let Ok(group) = group {
    //         groups.push(group);
    //     } else if let Err(err) = group {
    //         error_occurred = true;
    //         // Display customized error message
    //         match err {
    //             DotaoError::ReadError { path, source } => {
    //                 eprintln!(
    //                     "Error: Read error for group '{}': {}: '{}'.",
    //                     group_path,
    //                     source,
    //                     path.display()
    //                 );
    //             },
    //             other_err => eprintln!("Error: {}: '{}'", other_err,
    // group_path),         }
    //     }
    // }

    // if error_occurred {
    //     process::exit(1);
    // }

    // let home_path = env::var("HOME").unwrap_or_else(|err| {
    //     eprintln!("Unable to read env variable HOME: {}.", err);
    //     process::exit(1);
    // });
    // let home_path = PathBuf::from(home_path);

    // let interactive_mode = args.is_present("interactive_mode");
    // let overwrite_file = args.is_present("overwrite_file");
    // let overwrite_directory = args.is_present("overwrite_directory");
    // let overwrite_symlink = args.is_present("overwrite_symlink");

    // let link_behavior = LinkBehavior::new(
    //     interactive_mode,
    //     overwrite_file,
    //     overwrite_directory,
    //     overwrite_symlink,
    // );

    // let fake_run = args.is_present("fake-run");
    // if fake_run {
    //     println!("Fake run activated, no changes will be made.");
    // }

    // // println!("{:#?}", link_behavior);

    // let mut link_information = LinkInformation::new();
    // link_information.configure_behavior(link_behavior);
    // for group in groups {
    //     link_information.add_group(group);
    // }
    // link_information
    //     .prepare_linkage_to_home(&home_path)
    //     .unwrap_or_else(|err| {
    //         eprintln!("prepare_linkage_to_home error: {}.", err);
    //         process::exit(1);
    //     });

    // if link_information.critical_error_occurred() {
    //     link_information.show_errors();
    //     process::exit(1);
    // }

    // if fake_run {
    //     println!("Skiping link_information.proceed_and_link().");
    // } else {
    //     link_information.proceed_and_link().unwrap_or_else(|err| {
    //         eprintln!("Mds ocorreu um erro!!!!!!!!!!!!!!!!!!!!!!!");
    //         eprintln!("{}.", err);
    //     });
    // }

    // println!("{:#?}", link_information.link_behavior);
    // println!("{:#?}", link_information.payload);
}
