//! Flujo de ejemplo de punta a punta: *snapshot* inicial, primer *commit*, creación de un
//! archivo, `add` y segundo *commit*.
//!
//! Es genérico sobre el algoritmo de hash ([`HashStrategy`]), el almacén de objetos
//! ([`ObjectStore`]), las referencias ([`RefStore`]) y el directorio de trabajo
//! ([`WorkDirMut`]), así que funciona igual con los *backends* `mem` y `fs`. Crear el
//! repositorio en sí (ej. inicializar `.gust/`) depende del medio, por lo que queda a cargo
//! de quien construye los *backends* antes de llamar a [`run`].

use std::{collections::BTreeMap, error::Error};

use crate::{
    commit::create_commit, hash::HashStrategy, log::first_parent_log, objects::Meta,
    refs::RefStore, store::ObjectStore, workdir::WorkDirMut,
};

/// Ruta del archivo que el flujo crea entre ambos *commits*.
pub const NEW_FILE: &str = "hello.txt";

/// Ejecuta el flujo completo y devuelve los ids de los dos *commits* (inicial y segundo).
///
/// El directorio de trabajo debe tener al menos un archivo, porque el primer *commit* no
/// puede estar vacío ([`crate::commit::CommitError::NothingToCommit`]).
///
/// ## Ejemplo
/// ```ignore
/// // Mismo flujo, en RAM
/// let mut store = MemoryStore::default();
/// let mut refs = MemoryRefs::<Sha1>::new("main");
/// let mut workdir = MemoryWorkDir::default();
/// workdir.write_file("README.md", b"# gust\n")?;
/// let (first, second) = demo::run::<Sha1, _, _, _>(&mut store, &mut refs, &mut workdir)?;
/// ```
pub fn run<H, S, R, W>(
    store: &mut S,
    refs: &mut R,
    workdir: &mut W,
) -> Result<(H::Digest, H::Digest), Box<dyn Error + Send + Sync>>
where
    H: HashStrategy,
    S: ObjectStore,
    R: RefStore<H>,
    W: WorkDirMut,
{
    // Snapshot del directorio actual: guarda los blobs y arma el índice
    let mut index = workdir.snapshot::<H, S>(store)?;

    // Primer commit, sin padre
    let first = create_commit::<H, S, R>(store, refs, &index, "initial commit", empty_meta())?;
    println!("create commit {:?}\n\tinitial commit", first);

    // Crea un archivo nuevo en el directorio de trabajo
    workdir.write_file(NEW_FILE, b"Hello world\n")?;

    // Lo guarda en el store y lo anota en el index existente
    workdir.add::<H, S>(store, &mut index, NEW_FILE)?;

    // Segundo commit, con `first` como padre
    let second = create_commit::<H, S, R>(store, refs, &index, "Add hello.txt", empty_meta())?;
    println!("create commit {:?}\n\tAdd hello.txt", second);

    // Recorre el historial desde HEAD hasta el commit raíz
    for entry in first_parent_log::<H, S>(store, refs.resolve_head()?) {
        let (id, commit) = entry?;
        println!("read commit {:?}\n\t{}", id, commit.message);
    }

    Ok((first, second))
}

/// Metadatos vacíos, mientras no haya autor ni fecha configurables.
fn empty_meta() -> Meta {
    Meta {
        fields: BTreeMap::new(),
    }
}
