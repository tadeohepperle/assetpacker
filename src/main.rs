use std::env::args;

use gather::gather_assets;
use pack::pack_assets;

mod font;
mod gather;
mod pack;

fn main() {
    let args: Vec<String> = args().collect();
    let src_dir = args
        .get(1)
        .expect("Use like this: assetpacker path/to/srcdir path/to/destination");
    let dest_dir = args.get(2).cloned().unwrap_or(String::from("packed"));
    let assets = gather_assets(&src_dir);
    pack_assets(&assets, &dest_dir);
}
