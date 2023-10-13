#![allow(unused)]
#![feature(negative_impls)]

mod rc;
mod cell;

#[cfg(test)]
mod tests {
    use std::cell::{Cell, UnsafeCell};
    use std::ops::Deref;

    #[test]
    fn it_works() {
        let t = &[1, 2, 3];
        println!("{:#?}", t);
    }
}

