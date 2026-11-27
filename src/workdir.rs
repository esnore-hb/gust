//! Abstrae el **directorio de trabajo**: los archivos que el usuario edita y que se convierten
//! en un *snapshot* ([`Tree`]) al hacer *commit*.
//!
//! ## Convención de rutas
//! Las rutas son `&str` relativas a la raíz del directorio de trabajo, con componentes separados
//! por `'/'`, y `""` representa la raíz (ej. `"src/main.rs"`). Nunca son absolutas ni usan
//! `std::path::Path`: cada *backend* traduce estas rutas a su medio (disco, memoria, etc.).
//!
//! ## Diseño
//! Igual que [`ObjectStore`], el *backend* implementa solo lo mínimo ([`WorkDir::read_dir`] y
//! [`WorkDir::read_file`]) y obtiene gratis [`WorkDir::snapshot`], que recorre el directorio y
//! guarda sus [`Blob`] y [`Tree`].
//!
//! ## Limitaciones conocidas (post-MVP)
//! - **Solo archivos regulares y directorios:** los *symlinks* y otros tipos se omiten.
//! - **Sin caché de *stat*:** cada *snapshot* lee y hashea todos los archivos (Git usa un
//!   *index* con fechas y tamaños para saltarse archivos que no cambiaron).
//! - En [`build_tree`] se llama a write recursivamente por cada `item`. Si hay un error de escritura
//!   en uno de los items estos (en principio) permanecen escritos.

use std::{error::Error, fmt, io};

use crate::objects::{Blob, EntryKind, Tree, TreeEntry, validate_entry_name};
use crate::hash::HashStrategy;
use crate::store::{ObjectStore, StoreError};

/// # FileKind
/// Tipo de una entrada del directorio de trabajo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    /// Archivo regular
    File,
    /// Directorio
    Dir,
}

/// # DirEntry
/// Una entrada de un directorio de trabajo: su nombre (un único componente) y su tipo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntry {
    /// Nombre de la entrada, sin la ruta del directorio padre (ej. `"main.rs"`)
    pub name: String,
    /// Archivo o directorio
    pub kind: FileKind,
}

/// # WorkDirError
/// Error producido al leer o escribir en un [`WorkDir`].
#[derive(Debug)]
pub enum WorkDirError {
    /// Error de entrada/salida del *backend*
    Io(io::Error),
    /// La ruta indicada no existe
    NotFound(String),
    /// Un nombre o ruta no es válido como entrada de [`Tree`] (ej. `".."`) o choca con otra
    /// entrada (ej. un archivo con el mismo nombre que un directorio)
    InvalidName(String),
    /// Error al guardar objetos durante un *snapshot*
    Store(StoreError),
}

impl fmt::Display for WorkDirError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "error de I/O en el directorio de trabajo: {err}"),
            Self::NotFound(path) => write!(f, "ruta no encontrada: {path:?}"),
            Self::InvalidName(path) => write!(f, "nombre o ruta inválida: {path:?}"),
            Self::Store(err) => write!(f, "error al guardar el snapshot: {err}"),
        }
    }
}

impl Error for WorkDirError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Store(err) => Some(err),
            Self::NotFound(_) | Self::InvalidName(_) => None,
        }
    }
}

impl From<io::Error> for WorkDirError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<StoreError> for WorkDirError {
    fn from(err: StoreError) -> Self {
        Self::Store(err)
    }
}

/// Une la ruta de un directorio con el nombre de una entrada (`""` + `"a"` = `"a"`,
/// `"src"` + `"a"` = `"src/a"`), con una sola reserva de memoria.
pub(crate) fn join_path(dir: &str, name: &str) -> String {
    if dir.is_empty() {
        return name.to_owned();
    }
    let mut path = String::with_capacity(dir.len() + 1 + name.len());
    path.push_str(dir);
    path.push('/');
    path.push_str(name);
    path
}

/// # WorkDir
/// Define la lectura del directorio de trabajo, independiente del medio concreto.
///
/// El *backend* solo lista directorios y lee archivos; [`WorkDir::snapshot`] viene gratis.
pub trait WorkDir {
    /// Devuelve las entradas de `dir`, en cualquier orden.
    ///
    /// Debe **excluir las entradas ignoradas**, como mínimo la carpeta de metadatos del
    /// repositorio (ej. `.gust`), para no guardar el almacén dentro de sí mismo. Solo devuelve
    /// archivos regulares y directorios: otros tipos (*symlinks*, *sockets*, etc.) se omiten.
    fn read_dir(&self, dir: &str) -> Result<Vec<DirEntry>, WorkDirError>;

    /// Lee el contenido completo del archivo `path`.
    fn read_file(&self, path: &str) -> Result<Vec<u8>, WorkDirError>;

    /// Recorre el directorio de trabajo, guarda un [`Blob`] por archivo y un [`Tree`] por
    /// directorio en `store`, y devuelve el *digest* del [`Tree`] raíz.
    ///
    /// Los subdirectorios vacíos (o que solo contienen subdirectorios vacíos) se omiten, igual
    /// que en Git; la raíz sí puede quedar vacía. Como [`ObjectStore::write`] no reescribe lo
    /// que ya existe, solo se escriben los objetos nuevos.
    ///
    /// `where Self: Sized` mantiene el trait usable como `dyn WorkDir` pese a ser genérico.
    fn snapshot<H: HashStrategy, S: ObjectStore>(
        &self,
        store: &mut S,
    ) -> Result<H::Digest, WorkDirError>
    where
        Self: Sized,
    {
        let root = build_tree::<H, S, Self>(self, store, "")?;
        Ok(store.write(&root)?)
    }
}

/// Construye (sin guardarlo) el [`Tree`] de `dir`, guardando antes sus *blobs* y sus
/// subdirectorios no vacíos.
fn build_tree<H, S, W>(wd: &W, store: &mut S, dir: &str) -> Result<Tree<H>, WorkDirError>
where
    H: HashStrategy,
    S: ObjectStore,
    W: WorkDir + ?Sized,
{
    let listing = wd.read_dir(dir)?;
    let mut entries = Vec::with_capacity(listing.len());

    for item in listing {
        let path = join_path(dir, &item.name);
        if validate_entry_name(&item.name).is_err() {
            return Err(WorkDirError::InvalidName(path));
        }
        match item.kind {
            FileKind::File => {
                let blob = Blob::<H>::new(wd.read_file(&path)?);
                let hash = store.write(&blob)?;
                entries.push(TreeEntry {
                    kind: EntryKind::Blob,
                    name: item.name,
                    hash,
                });
            }
            FileKind::Dir => {
                let subtree = build_tree::<H, S, W>(wd, store, &path)?;
                // Git no versiona directorios vacíos: no se guarda ni se referencia
                if subtree.entries.is_empty() {
                    continue;
                }
                let hash = store.write(&subtree)?;
                entries.push(TreeEntry {
                    kind: EntryKind::Tree,
                    name: item.name,
                    hash,
                });
            }
        }
    }

    // Ordena y detecta nombres repetidos (un backend que lista dos veces la misma entrada)
    Tree::from_entries(entries).map_err(|_| WorkDirError::InvalidName(dir.to_owned()))
}

/// # WorkDirMut
/// Extiende [`WorkDir`] con escritura (**post-MVP**, solo firmas).
///
/// Servirá para restaurar (*checkout*) un [`Tree`] en el directorio de trabajo. Antes de
/// escribir habrá que calcular un diff entre `HEAD`, el destino y el directorio actual para no
/// pisar cambios locales sin guardar.
pub trait WorkDirMut: WorkDir {
    /// Crea o reemplaza el archivo `path` con `content`, creando los directorios padres.
    fn write_file(&mut self, path: &str, content: &[u8]) -> Result<(), WorkDirError>;

    /// Elimina el archivo `path` y los directorios padres que queden vacíos.
    fn remove_file(&mut self, path: &str) -> Result<(), WorkDirError>;
}
