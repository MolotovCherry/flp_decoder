#![feature(drain_filter)]

mod flp_format;

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() == 2 {
        let flp = flp_format::FLP::read(&args[1]);
        println!("{:?}", flp);
    } else {
        println!("flp_unlocker <path_to_project.flp>");
    }
}
