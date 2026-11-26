mod store;
mod working_tree;
mod hash;
mod store2;
mod refs;

mod objects;

use objects::{Blob};

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let blob = Blob::new(b"hello".to_vec());
    //println!("{:?}", blob);

    let mut wt = working_tree::WorkingTree::open(".")?;
    wt.add("README.md")?;
    println!("{:?}", wt.files());

    Ok(())
}
