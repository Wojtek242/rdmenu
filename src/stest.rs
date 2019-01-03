//! stest
//!
//! Filter a list of files by properties.

// External crates
extern crate libc;

// std imports
use std::error::Error;
use std::ffi::CString;
use std::fs::Metadata;
use std::os::linux::fs::MetadataExt;
use std::path::PathBuf;
use std::time::SystemTime;

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
    block_special: bool,

    /// Test that files are character specials
    #[structopt(short = "c")]
    char_special: bool,

    /// Test that files are directories
    #[structopt(short = "d")]
    directory: bool,

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
    fifo: bool,

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

    /// List of files
    #[structopt(parse(from_os_str))]
    files: Vec<PathBuf>,
}

/// Reduce method for accumulating stest outcomes if multiple files are tested.
fn reduce(acc: bool, x: bool) -> bool {
    acc || x
}

/// Run stest with the provided options. Returns an error if it cannot open the
/// `newer_than` or `older_than` files.
pub fn run(opt: &Opt) -> Result<bool, std::io::Error> {
    let compare = if let Some(ref file_path) = opt.newer_than {
        Some(file_path.metadata()?.modified()?)
    } else if let Some(ref file_path) = opt.older_than {
        Some(file_path.metadata()?.modified()?)
    } else {
        None
    };

    let stest = Stest { opt, compare };

    Ok(stest.run())
}

/// Struct representing current stest instance.
struct Stest<'t> {
    opt: &'t Opt,
    compare: Option<SystemTime>,
}

impl<'t> Stest<'t> {
    /// Run stest.
    fn run(&self) -> bool {
        if self.opt.files.is_empty() {
            self.run_stdin()
        } else {
            self.run_opts()
        }
    }

    /// Take input from stdin - currently not supported.
    fn run_stdin(&self) -> bool {
        panic!("Read from stdin");
    }

    /// Take input from the files passed in the options.
    fn run_opts(&self) -> bool {
        let iter = self.opt.files.iter();

        if self.opt.dir_contents {
            iter.map(|path| self.test_dir(&path)).fold(false, reduce)
        } else {
            iter.map(|path| {
                path.to_str()
                    .and_then(|file_name| Some(self.test(&path, file_name)))
                    .unwrap_or(false)
            }).fold(false, reduce)
        }
    }

    /// Test the contents of a directory.
    fn test_dir(&self, dir_path: &PathBuf) -> bool {
        if let Ok(dir) = dir_path.read_dir() {
            let dir_contents = dir.filter_map(|path_result| {
                path_result.ok().and_then(|path| Some(path.path()))
            });

            return dir_contents
                .map(|path| {
                    path.file_name()
                        .and_then(|os_str| os_str.to_str())
                        .and_then(|file_name| Some(self.test(&path, file_name)))
                        .unwrap_or(false)
                })
                .fold(false, reduce);
        }

        false
    }

    /// Test the provided file.
    fn test(&self, path: &PathBuf, file_name: &str) -> bool {

        let file = path.metadata();
        let c_path = path.to_str().and_then(|path| CString::new(path).ok());

        // The test outcome.
        let mut result = false;

        // Check if file is accessible.
        if file.is_ok() && c_path.is_some() {
            let file = file.unwrap();
            let c_path = c_path.unwrap();

            // If file is accessible test it.
            result = (self.opt.hidden || !file_name.starts_with('.')) &&
                (!self.opt.block_special || s_isval(libc::S_IFBLK, &file)) &&
                (!self.opt.char_special || s_isval(libc::S_IFCHR, &file)) &&
                (!self.opt.directory || file.is_dir()) &&
                (!self.opt.regular || file.is_file()) &&
                (!self.opt.set_gid_set || s_isset(libc::S_ISGID, &file)) &&
                (!self.opt.symbolic_link || is_symlink(path)) &&
                (!self.opt.newer_than.is_some() ||
                     (self.compare.unwrap() < file.modified().unwrap())) &&
                (!self.opt.older_than.is_some() ||
                     (file.modified().unwrap() < self.compare.unwrap())) &&
                (!self.opt.fifo || s_isval(libc::S_IFIFO, &file)) &&
                (!self.opt.readable || access(libc::R_OK, &c_path)) &&
                (!self.opt.not_empty || (file.len() > 0)) &&
                (!self.opt.set_gid_set || s_isset(libc::S_ISUID, &file)) &&
                (!self.opt.writable || access(libc::W_OK, &c_path)) &&
                (!self.opt.executable || access(libc::X_OK, &c_path));
        }

        // Invert result if necessary.
        result ^= self.opt.invert;

        // Print successful result unless asked not to.
        if result && !self.opt.quiet {
            println!("{}", file_name);
        }

        result

    }
}

/// Utility function to provide the function of the libc macros such as ISBLK,
/// ISCHR, ISFIFO.
fn s_isval(s_ifval: u32, file: &Metadata) -> bool {
    (file.st_mode() & libc::S_IFMT) == s_ifval
}

/// Utility function to check the flags if the file's mode.
fn s_isset(s_isflg: i32, file: &Metadata) -> bool {
    (file.st_mode() & s_isflg as u32) != 0
}

/// Utility function to check if file at path is a symlink.
fn is_symlink(path: &PathBuf) -> bool {
    path.symlink_metadata()
        .ok()
        .filter(|link| link.file_type().is_symlink())
        .is_some()
}

/// Wrapper around libc's unsafe access call.
fn access(rwx: i32, c_path: &CString) -> bool {
    (unsafe { libc::access(c_path.as_ptr(), rwx) } == 0)
}
