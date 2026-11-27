//! Abstrae el **directorio de trabajo**: los archivos que el usuario edita y que se convierten
//! en un *index* (`ruta -> id del blob`) que [`crate::commit::create_commit`] registra.
//!
//! ## Convención de rutas
//! Las rutas son `&str` relativas a la raíz del directorio de trabajo, con componentes separados
//! por `'/'`, y `""` representa la raíz (ej. `"src/main.rs"`). Nunca son absolutas ni usan
//! `std::path::Path`: cada *backend* traduce estas rutas a su medio (disco, memoria, etc.).
//!
//! ## Diseño
//! Igual que [`ObjectStore`], el *backend* implementa solo lo mínimo ([`WorkDir::read_dir`] y
//! [`WorkDir::read_file`]) y obtiene gratis [`WorkDir::snapshot`], que recorre el directorio,
//! guarda sus [`Blob`] y devuelve el *index*. Los [`Tree`](crate::objects::Tree) los
//! construye solo [`crate::commit::build_tree`], a partir de ese *index*.
//!
//! ## Limitaciones conocidas (post-MVP)
//! - **Solo archivos regulares y directorios:** los *symlinks* y otros tipos se omiten.
//! - **Sin caché de *stat*:** cada *snapshot* lee y hashea todos los archivos (Git usa un
//!   *index* con fechas y tamaños para saltarse archivos que no cambiaron).
//! - Si una escritura falla a mitad del recorrido, los [`Blob`] ya guardados permanecen en el
//!   almacén (sin nada que los referencie).

use std::{collections::BTreeMap, error::Error, fmt, io};

use crate::hash::HashStrategy;
use crate::objects::{Blob, validate_entry_name};
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
    /// Un nombre o ruta no es válido como entrada de [`Tree`](crate::objects::Tree)
    /// (ej. `".."`) o choca con otra entrada (ej. un archivo listado dos veces)
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

    /// Recorre el directorio de trabajo, guarda un [`Blob`] por archivo en `store` y devuelve
    /// el *index* resultante (`ruta -> id del blob`), listo para [`crate::commit::create_commit`].
    ///
    /// Los directorios vacíos no aparecen en el *index*, así que no llegan al *commit*. Como
    /// [`ObjectStore::write`] no reescribe lo que ya existe, solo se escriben los *blobs* nuevos.
    ///
    /// `where Self: Sized` mantiene el trait usable como `dyn WorkDir` pese a ser genérico.
    fn snapshot<H: HashStrategy, S: ObjectStore>(
        &self,
        store: &mut S,
    ) -> Result<BTreeMap<String, H::Digest>, WorkDirError>
    where
        Self: Sized,
    {
        let mut index = BTreeMap::new();
        collect_index::<H, S, Self>(self, store, "", &mut index)?;
        Ok(index)
    }

    /// Añade archivo en `path` a un índice existente y lo guarda en el `store`
    fn add<H: HashStrategy, S: ObjectStore>(
        &self,
        store: &mut S,
        index: &mut BTreeMap<String, H::Digest>,
        path: &str,
    ) -> Result<(), WorkDirError>
    where
        Self: Sized,
    {
        if path.split('/').any(|part| validate_entry_name(part).is_err()) {
            return Err(WorkDirError::InvalidName(path.to_owned()));
        }
        let hash = store.write(&Blob::<H>::new(self.read_file(path)?))?;
        index.insert(path.to_owned(), hash);
        Ok(())
    }
}

/// Guarda los *blobs* de `dir` (y de sus subdirectorios) y anota sus rutas en `index`.
fn collect_index<H, S, W>(
    wd: &W,
    store: &mut S,
    dir: &str,
    index: &mut BTreeMap<String, H::Digest>,
) -> Result<(), WorkDirError>
where
    H: HashStrategy,
    S: ObjectStore,
    W: WorkDir + ?Sized,
{
    for item in wd.read_dir(dir)? {
        let path = join_path(dir, &item.name);
        if validate_entry_name(&item.name).is_err() {
            return Err(WorkDirError::InvalidName(path));
        }
        match item.kind {
            FileKind::File => {
                // Un backend que lista dos veces la misma entrada
                if index.contains_key(&path) {
                    return Err(WorkDirError::InvalidName(path));
                }
                let hash = store.write(&Blob::<H>::new(wd.read_file(&path)?))?;
                index.insert(path, hash);
            }
            FileKind::Dir => collect_index::<H, S, W>(wd, store, &path, index)?,
        }
    }
    Ok(())
}

/// # WorkDirMut
/// Extiende [`WorkDir`] con escritura (**post-MVP**, solo firmas).
///
/// Servirá para restaurar (*checkout*) un [`Tree`](crate::objects::Tree) en el directorio de
/// trabajo. Antes de escribir habrá que calcular un diff entre `HEAD`, el destino y el
/// directorio actual para no pisar cambios locales sin guardar.
pub trait WorkDirMut: WorkDir {
    /// Crea o reemplaza el archivo `path` con `content`, creando los directorios padres.
    fn write_file(&mut self, path: &str, content: &[u8]) -> Result<(), WorkDirError>;

    /// Elimina el archivo `path` y los directorios padres que queden vacíos.
    fn remove_file(&mut self, path: &str) -> Result<(), WorkDirError>;
}
