use std::path::PathBuf;

/// Filter a list of files by properties. `stest` takes a list of files and
/// filters by the files' properties, analogous to `test`(1).  Files which pass
/// all tests are printed to stdout. If no files are given, stest reads files
/// from stdin.

#[derive(Debug, StructOpt)]
#[structopt(name = "stest", raw(global_settings = "&[clap::AppSettings::DeriveDisplayOrder]"))]
pub struct Opt {
    /// Test hidden files
    #[structopt(short = "a")]
    hidden: bool,

    /// Test that files are block specials
    #[structopt(short = "b")]
    block: bool,

    /// Test that files are character specials
    #[structopt(short = "c")]
    char_special: bool,

    /// Test that files are directories
    #[structopt(short = "d")]
    directory: bool,

    /// Test that files exist
    #[structopt(short = "e")]
    exists: bool,

    /// Test that files are regular files
    #[structopt(short = "f")]
    regular: bool,

    /// Test that files have their set-group-ID flag set
    #[structopt(short = "g")]
    set_gid_set: bool,

    /// Test that files are symbolic links
    #[structopt(short = "h")]
    symbolic_link: bool,

    /// Test the contents of a directory given as an argument
    #[structopt(short = "l")]
    dir_contents: bool,

    /// Test that files are newer than file
    #[structopt(short = "n", parse(from_os_str))]
    newer_than: Option<PathBuf>,

    /// Test that files are older than file
    #[structopt(short = "o", parse(from_os_str))]
    older_than: Option<PathBuf>,

    /// Test that files are named pipes
    #[structopt(short = "p")]
    pipes: bool,

    /// No files are printed, only the exit status is returned
    #[structopt(short = "q")]
    quiet: bool,

    /// Test that files are readable
    #[structopt(short = "r")]
    readable: bool,

    /// Test that files are not empty
    #[structopt(short = "s")]
    not_empty: bool,

    /// Test that files have their set-user-ID flag set
    #[structopt(short = "u")]
    set_uid_set: bool,

    /// Invert the sense of tests, only failing files pass
    #[structopt(short = "v")]
    invert: bool,

    /// Test that files are writable
    #[structopt(short = "w")]
    writable: bool,

    /// Test that files are executable
    #[structopt(short = "x")]
    executable: bool,
}
