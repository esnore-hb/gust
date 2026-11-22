mod objects;
mod store;
mod working_tree;
mod hash;

use crate::objects::Blob;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let blob = Blob { content: b"hello".to_vec() };
    println!("{:?}", blob);

    let mut wt = working_tree::WorkingTree::open(".")?;
    wt.add("README.md")?;
    println!("{:?}", wt.files());

    Ok(())
}
