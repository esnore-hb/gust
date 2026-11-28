mod store;
mod hash;
mod commit;
mod demo;
mod log;
mod refs;
mod workdir;

mod objects;
mod backends;

use backends::{
    hash::Sha1,
    mem::{MemoryRefs, MemoryStore, MemoryWorkDir},
};
use color_eyre::eyre::eyre;
use workdir::WorkDirMut;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    // Backends en RAM; cambiar a fs solo cambia estas líneas
    let mut store = MemoryStore::default();
    let mut refs = MemoryRefs::<Sha1>::new("main");
    let mut workdir = MemoryWorkDir::default();
    workdir.write_file("README.md", b"# gust\n")?;

    let (first, second) =
        demo::run::<Sha1, _, _, _>(&mut store, &mut refs, &mut workdir).map_err(|e| eyre!(e))?;
    println!("initial: {first:?}\nsegundo: {second:?}");
    Ok(())
}
