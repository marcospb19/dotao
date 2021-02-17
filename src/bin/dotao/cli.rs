use clap::{crate_name, crate_version, App, AppSettings, Arg, SubCommand};

// Why isn't this working as intended?
// .about("See --help for more detailed help.")
// .long_about("See -h for shorter help.")
pub fn parse_args() -> clap::ArgMatches<'static> {
    App::new(crate_name!())
        .version(crate_version!())
        .settings(&[AppSettings::ColoredHelp, AppSettings::ArgRequiredElseHelp])
        .help_message("Display help information.")
        .version_message("Display version information.")
        .subcommand(
            SubCommand::with_name("add")
                .settings(&[AppSettings::ColoredHelp])
                .arg(
                    Arg::with_name("groups")
                        .required(true)
                        .multiple(true)
                        .help("Groups directories."),
                )
                .aliases(&["a", "insert"])
                .about("Add groups to the tree file."),
        )
        .subcommand(
            SubCommand::with_name("remove")
                .settings(&[AppSettings::ColoredHelp])
                .arg(
                    Arg::with_name("groups")
                        .required(true)
                        .multiple(true)
                        .help("Groups directories."),
                )
                .aliases(&["r", "rm", "delete"])
                .about("Remove groups from the tree file."),
        )
        .subcommand(SubCommand::with_name("link")
                // .arg(
                //     Arg::with_name("groups")
                //         .multiple(true)
                //         .help("Group folders to remove from the tree (default=all)"),
                // )
                .alias("l")
                .about("Link groups in the tree file."))
                .settings(&[AppSettings::ColoredHelp])
        .subcommand(
            SubCommand::with_name("unlink")
                // .arg(
                //     Arg::with_name("groups")
                //         .multiple(true)
                //         .help("Group folders to remove from the tree (default=all)"),
                // )
                .alias("u")
                .settings(&[AppSettings::ColoredHelp])
                .about("Unlink groups in the tree file."),
        )
        .subcommand(
            SubCommand::with_name("init")
                .settings(&[AppSettings::ColoredHelp])
                .arg(
                    Arg::with_name("force")
                        .long("force")
                        .short("f")
                        .help("Create even if there's no git repository"),
                )
                .alias("i")
                .about("Create tree file."),
        )
        .get_matches()
}
// .arg(
//     Arg::with_name("interactive_mode")
//         .short("i")
//         .help("Run in interactive mode, try to solve conflicts with questions,"),
// )
// .arg(
//     Arg::with_name("overwrite_symlink")
//         .short("s")
//         .help("Overwrite symlinks."),
// )
// .arg(
//     Arg::with_name("overwrite_file")
//         .short("f")
//         .help("Overwrite files."),
// )
// .arg(
//     Arg::with_name("overwrite_directory")
//         .short("d")
//         .help("Overwrite directories."),
// )
// .arg(
//     Arg::with_name("fake-run")
//         .long("fake")
//         .help("Don't mess with files."),
// )
