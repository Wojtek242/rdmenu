extern crate libc;

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

        let file = path.metadata();
        let c_path = path.to_str().and_then(|path| CString::new(path).ok());
        let file_name = match path.file_name() {
            Some(os_str) => os_str.to_str(),
            None => {
                path.to_str().and_then(|path_str| {
                    path_str.split(std::path::MAIN_SEPARATOR).last()
                })
            }
        };


        let mut result = file.is_ok() && c_path.is_some() && file_name.is_some();

        if result {
            // If `result` is true, all three are guaranteed to unwrap.
            let file = file.unwrap();
            let c_path = c_path.unwrap();
            let file_name = file_name.unwrap();

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

        result ^= self.opt.invert;

        if result && !self.opt.quiet {
            if let Some(name) = file_name {
                println!("{}", name);
            } else if let Some(name) = path.to_str() {
                println!("{}", name);
            }
        }

        result

    }
}

fn s_isval(s_ifval: u32, file: &Metadata) -> bool {
    (file.st_mode() & libc::S_IFMT) == s_ifval
}

fn s_isset(s_isflg: i32, file: &Metadata) -> bool {
    (file.st_mode() & s_isflg as u32) != 0
}

fn is_symlink(path: &PathBuf) -> bool {
    path.symlink_metadata()
        .ok()
        .filter(|link| link.file_type().is_symlink())
        .is_some()
}

fn access(rwx: i32, c_path: &CString) -> bool {
    (unsafe { libc::access(c_path.as_ptr(), rwx) } == 0)
}
