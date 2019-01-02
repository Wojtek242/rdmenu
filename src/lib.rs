#[macro_use]
extern crate structopt;

pub mod stest;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
