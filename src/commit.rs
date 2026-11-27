//! Crea un *commit* a partir del *index*: construye el [`Tree`] del proyecto, guarda el
//! [`Commit`] y avanza la rama a la que apunta `HEAD`.
//!
//! Es genérico sobre el algoritmo de hash ([`HashStrategy`]), el almacenamiento de objetos
//! ([`ObjectStore`]) y el de referencias ([`RefStore`]), por lo que no depende de ningún
//! *backend* concreto.
//!
//! El *index* se recibe como un mapa `ruta -> id del blob`, con rutas relativas a la raíz
//! del repositorio y separadas por `/` (ej. `src/main.rs`). Los [`crate::core::Blob`]
//! referenciados deben estar guardados antes de crear el *commit*.

use std::{
    collections::{BTreeMap, btree_map},
    error::Error,
    fmt,
};

use crate::{
    objects::{Commit, EntryKind, Meta, Tree, TreeEntry},
    hash::HashStrategy,
    refs::{RefError, RefStore},
    store::{ObjectStore, StoreError},
};

/// # CommitError
/// Error producido al construir el *tree* o al registrar un [`Commit`].
#[derive(Debug)]
#[non_exhaustive]
pub enum CommitError {
    /// El mensaje es vacío o solo contiene espacios
    EmptyMessage,
    /// El *index* no tiene cambios respecto al último *commit*
    NothingToCommit,
    /// Un metadato rompería el formato `<key> <value>\n`
    InvalidMeta(String),
    /// Una ruta del *index* no es una ruta relativa válida
    InvalidPath(String),
    /// Una misma ruta aparece como archivo y como directorio
    PathConflict(String),
    /// El *index* referencia un *blob* que no está en el almacenamiento
    MissingBlob(String),
    /// Error al leer o escribir objetos
    Store(StoreError),
    /// Error al consultar o actualizar referencias
    Ref(RefError),
}

impl fmt::Display for CommitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyMessage => write!(f, "el mensaje del commit está vacío"),
            Self::NothingToCommit => write!(f, "no hay cambios para registrar"),
            Self::InvalidMeta(key) => write!(f, "metadato inválido: {key:?}"),
            Self::InvalidPath(path) => write!(f, "ruta inválida en el index: {path:?}"),
            Self::PathConflict(path) => {
                write!(f, "la ruta {path:?} es archivo y directorio a la vez")
            }
            Self::MissingBlob(path) => write!(f, "falta el blob del archivo {path:?}"),
            // StoreError todavía no implementa Display
            Self::Store(err) => write!(f, "error en el almacenamiento de objetos: {err:?}"),
            Self::Ref(err) => write!(f, "{err}"),
        }
    }
}

impl Error for CommitError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Ref(err) => Some(err),
            _ => None,
        }
    }
}

impl From<StoreError> for CommitError {
    fn from(err: StoreError) -> Self {
        Self::Store(err)
    }
}

impl From<RefError> for CommitError {
    fn from(err: RefError) -> Self {
        Self::Ref(err)
    }
}

/// # Node
/// Nodo intermedio que agrupa las rutas planas del *index* en directorios.
///
/// Toma prestados los nombres y *digests* del *index*, sin copiarlos.
enum Node<'a, D> {
    /// Archivo, con el id de su *blob*
    File(&'a D),
    /// Directorio, con sus hijos ordenados por nombre
    Dir(BTreeMap<&'a str, Node<'a, D>>),
}

/// Inserta `path` dentro de `root`, creando los directorios intermedios.
fn insert_path<'a, D>(
    root: &mut BTreeMap<&'a str, Node<'a, D>>,
    path: &'a str,
    digest: &'a D,
) -> Result<(), CommitError> {
    // "" cubre las rutas vacías, absolutas, con "//" o terminadas en "/".
    // `\0` se rechaza porque es el separador dentro de un TreeEntry serializado.
    if path
        .split('/')
        .any(|part| matches!(part, "" | "." | "..") || part.contains('\0'))
    {
        return Err(CommitError::InvalidPath(path.to_owned()));
    }

    let mut dir = root;
    let mut rest = path;
    while let Some((name, tail)) = rest.split_once('/') {
        let node = dir
            .entry(name)
            .or_insert_with(|| Node::Dir(BTreeMap::new()));
        dir = match node {
            Node::Dir(children) => children,
            Node::File(_) => return Err(CommitError::PathConflict(path.to_owned())),
        };
        rest = tail;
    }

    match dir.entry(rest) {
        btree_map::Entry::Vacant(slot) => {
            slot.insert(Node::File(digest));
            Ok(())
        }
        btree_map::Entry::Occupied(_) => Err(CommitError::PathConflict(path.to_owned())),
    }
}

/// Guarda el [`Tree`] de un directorio, escribiendo antes sus subdirectorios.
fn write_dir<H: HashStrategy, S: ObjectStore>(
    store: &mut S,
    children: &BTreeMap<&str, Node<'_, H::Digest>>,
) -> Result<H::Digest, CommitError> {
    let mut entries = Vec::with_capacity(children.len());
    for (&name, node) in children {
        let (kind, hash) = match node {
            Node::File(digest) => (EntryKind::Blob, (*digest).clone()),
            Node::Dir(grandchildren) => (EntryKind::Tree, write_dir::<H, S>(store, grandchildren)?),
        };
        entries.push(TreeEntry::<H> {
            kind,
            name: name.to_owned(),
            hash,
        });
    }
    Ok(store.write(&Tree::<H> { entries })?)
}

/// Construye y guarda los [`Tree`] que representan el *index*, y devuelve el id del raíz.
///
/// Todas las rutas y *blobs* se validan antes de escribir el primer objeto. Un *index*
/// vacío produce un [`Tree`] sin entradas.
pub fn build_tree<H: HashStrategy, S: ObjectStore>(
    store: &mut S,
    index: &BTreeMap<String, H::Digest>,
) -> Result<H::Digest, CommitError> {
    let mut root = BTreeMap::new();
    for (path, digest) in index {
        insert_path(&mut root, path, digest)?;
        if !store.contains(digest.as_ref())? {
            return Err(CommitError::MissingBlob(path.clone()));
        }
    }
    write_dir::<H, S>(store, &root)
}

/// Verifica que los metadatos se puedan volver a leer tal como se escribieron.
fn validate_meta(meta: &Meta) -> Result<(), CommitError> {
    let invalid = meta
        .fields
        .iter()
        .find(|(key, value)| key.is_empty() || key.contains([' ', '\n']) || value.contains('\n'));
    match invalid {
        Some((key, _)) => Err(CommitError::InvalidMeta(key.clone())),
        None => Ok(()),
    }
}

/// Registra el estado del *index* como un nuevo [`Commit`] y devuelve su id.
///
/// El *commit* tiene como padre al actual de `HEAD` (ninguno si es el primero). La rama
/// avanza solo al final y solo si nadie la movió entretanto, así que ante un error el
/// historial queda intacto.
///
/// No crea *commits* de *merge*.
///
/// ## Ejemplo
/// ```ignore
/// // El blob se guarda antes y su id se anota en el index
/// let blob_id = store.write(&Blob::<HashKetchum>::new(b"hola".to_vec()))?;
/// let index = BTreeMap::from([(String::from("docs/hello.txt"), blob_id)]);
///
/// let meta = Meta { fields: BTreeMap::new() };
/// let id = create_commit::<HashKetchum, _, _>(&mut store, &mut refs, &index, "Saluda", meta)?;
/// ```
pub fn create_commit<H, S, R>(
    store: &mut S,
    refs: &mut R,
    index: &BTreeMap<String, H::Digest>,
    message: &str,
    meta: Meta,
) -> Result<H::Digest, CommitError>
where
    H: HashStrategy,
    S: ObjectStore,
    R: RefStore<H>,
{
    if message.trim().is_empty() {
        return Err(CommitError::EmptyMessage);
    }
    validate_meta(&meta)?;

    let parent = refs.resolve_head()?;
    // El primer commit no puede estar vacío
    if parent.is_none() && index.is_empty() {
        return Err(CommitError::NothingToCommit);
    }

    let root_hash = build_tree::<H, S>(store, index)?;
    // Mismo tree raíz que el padre: no cambió ningún archivo
    if let Some(parent_id) = &parent {
        let previous: Commit<H> = store.read(parent_id)?;
        if previous.root_hash == root_hash {
            return Err(CommitError::NothingToCommit);
        }
    }

    let commit = Commit::<H> {
        root_hash,
        parents: parent.iter().cloned().collect(),
        message: message.to_owned(),
        meta,
    };
    let id = store.write(&commit)?;

    let summary = message.trim().lines().next().unwrap_or_default();
    refs.advance_head(parent.as_ref(), id.clone(), &format!("commit: {summary}"))?;

    Ok(id)
}
