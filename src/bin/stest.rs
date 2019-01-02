extern crate rdmenu;
use rdmenu::stest;

extern crate structopt;
use structopt::StructOpt;

fn main() {
    stest::run(&stest::Opt::from_args()).expect("stest failed");
}
