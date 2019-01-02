extern crate rdmenu;
use rdmenu::stest::*;

extern crate structopt;
use structopt::StructOpt;

fn main() {
    let opt = Opt::from_args();
    println!("{:?}", opt);
}
