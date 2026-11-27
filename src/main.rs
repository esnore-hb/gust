mod store;
mod hash;
mod commit;
mod refs;
mod workdir;

mod objects;

use objects::{Blob};

fn main() -> color_eyre::Result<()> {
    /** 
    color_eyre::install()?;
    let blob = Blob::new(b"hello".to_vec());
    //println!("{:?}", blob);

    let mut wt = Workdir::WorkingTree::open(".")?;
    wt.add("README.md")?;
    println!("{:?}", wt.files());
    */
    Ok(())
    
}
