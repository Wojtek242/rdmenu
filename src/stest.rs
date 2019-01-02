extern crate libc;

use std::error::Error;
use std::ffi::CString;
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

fn reduce(acc: bool, x: bool) -> bool {
    acc || x
}

pub fn run(opt: &Opt) -> Result<bool, Box<Error>> {
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

struct Stest<'t> {
    opt: &'t Opt,
    compare: Option<SystemTime>,
}

impl<'t> Stest<'t> {
    fn run(&self) -> bool {
        if self.opt.files.is_empty() {
            self.run_stdin()
        } else {
            self.run_opts()
        }
    }

    fn run_stdin(&self) -> bool {
        panic!("Read from stdin");
    }

    fn run_opts(&self) -> bool {
        let iter = self.opt.files.iter();

        if self.opt.dir_contents {
            iter.map(|path| self.test_dir(&path)).fold(false, reduce)
        } else {
            iter.map(|path| self.test(&path)).fold(false, reduce)
        }
    }

    fn test_dir(&self, dir_path: &PathBuf) -> bool {
        if let Ok(dir) = dir_path.read_dir() {
            let dir_contents = dir.filter_map(|path_result| {
                path_result.ok().and_then(|path| Some(path.path()))
            });

            return dir_contents.map(|file| self.test(&file)).fold(
                false,
                reduce,
            );
        }

        false
    }

    fn test(&self, path: &PathBuf) -> bool {

        // --------------------------------------------------------------------
        // Extract file information.
        // --------------------------------------------------------------------

        let file = match path.metadata() {
            Ok(meta) => meta,
            Err(_) => return false,
        };

        let file_name = match path.file_name().and_then(|os_str| os_str.to_str()) {
            Some(name) => name,
            None => return false,
        };

        let c_path = match path.to_str().and_then(|path| CString::new(path).ok()) {
            Some(path) => path,
            None => return false,
        };

        // --------------------------------------------------------------------
        // Run the checks.
        // --------------------------------------------------------------------

        if !self.opt.hidden && file_name.starts_with('.') {
            return false; // Exclude hidden files
        }

        if self.opt.block_special && (file.st_mode() & libc::S_IFMT) == libc::S_IFBLK {
            return false; // Not block special
        }

        if self.opt.char_special && !((file.st_mode() & libc::S_IFMT) == libc::S_IFCHR) {
            return false; // Not char special
        }

        if self.opt.directory && !file.is_dir() {
            return false; // Not a directory
        }

        if self.opt.regular && !file.is_file() {
            return false; // Not a file
        }

        if self.opt.set_gid_set && !((file.st_mode() & libc::S_ISGID as u32) != 0) {
            return false; // Set GID flag unset
        }

        if self.opt.symbolic_link &&
            !path.symlink_metadata()
                .ok()
                .filter(|link| link.file_type().is_symlink())
                .is_some()
        {
            return false;
        }

        // modified() only returns an error if it is not supported on the
        // current platform and since we already have the time for the file we
        // are comparing against then we already know it must be supported.

        if self.opt.newer_than.is_some() && !(self.compare.unwrap() < file.modified().unwrap()) {
            return false; // Older than other file
        }

        if self.opt.older_than.is_some() && !(file.modified().unwrap() < self.compare.unwrap()) {
            return false; // Newer than other file
        }

        if self.opt.fifo && !((file.st_mode() & libc::S_IFMT) == libc::S_IFIFO) {
            return false; // Not a named pipe
        }

        if self.opt.readable && !(unsafe { libc::access(c_path.as_ptr(), libc::R_OK) } == 0) {
            return false; // Not readable
        }

        if self.opt.not_empty && !(file.len() > 0) {
            return false; // Empty
        }

        if self.opt.set_gid_set && !((file.st_mode() & libc::S_ISUID as u32) != 0) {
            return false; // Set UID flag unset
        }

        if self.opt.writable && !(unsafe { libc::access(c_path.as_ptr(), libc::W_OK) } == 0) {
            return false; // Not writable
        }

        if self.opt.executable && !(unsafe { libc::access(c_path.as_ptr(), libc::X_OK) } == 0) {
            return false; // Not executable
        }

        // --------------------------------------------------------------------
        // Print files that passed all checks.
        // --------------------------------------------------------------------

        println!("{}", file_name);
        true

    }
}
