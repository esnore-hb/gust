//! *Backends* que guardan el repositorio en disco: los metadatos en la carpeta `.gust/`, en la
//! raíz del directorio de trabajo, y los archivos del usuario junto a ella.
//!
//! ## Layout de `.gust/`
//! ```text
//! <raíz>/
//! ├── .gust/
//! │   ├── HEAD            # nombre de la rama activa, en una línea (ej. "main\n")
//! │   ├── objects/<hex>   # un archivo por objeto, nombrado con su id en hexadecimal
//! │   ├── refs/<name>     # digest en hexadecimal de cada referencia; "a/b" es una subcarpeta
//! │   └── logs/refs       # registro append-only de cada cambio de referencia y su razón
//! └── ...                 # archivos del usuario (FsWorkDir)
//! ```
//!
//! Todas las rutas salen de [`Layout`], para que ningún *backend* arme rutas por su cuenta.
//!
//! ## Escritura atómica
//! Escribir un archivo "en su lugar" deja, si el proceso se corta a la mitad, un archivo
//! truncado: un objeto corrupto o una rama que apunta a basura. [`write_atomic`] escribe primero
//! en un archivo temporal de la misma carpeta y luego lo renombra sobre el destino. En un mismo
//! sistema de archivos, `rename` reemplaza el destino de una vez: quien lee ve el contenido
//! anterior o el nuevo, nunca uno a medias.
//!
//! ## Limitaciones conocidas (post-MVP)
//! - **`objects/` plano:** con decenas de miles de objetos, una sola carpeta se vuelve lenta en
//!   algunos sistemas de archivos. Git reparte los objetos en 256 subcarpetas (`ab/cdef…`);
//!   conviene hacerlo solo si se nota el costo.
//! - **Durabilidad ante cortes de luz:** `write_atomic` sincroniza el archivo temporal, pero no
//!   el directorio padre, así que tras un corte el `rename` podría no haber quedado registrado.

mod refs;
mod store;
mod workdir;

pub use refs::FsRefs;
pub use store::FsStore;
pub use workdir::FsWorkDir;

use std::{
    io,
    path::{Path, PathBuf},
};

/// Nombre de la carpeta de metadatos dentro de la raíz del directorio de trabajo.
///
/// [`FsWorkDir`] la excluye de sus listados para no guardar el repositorio dentro de sí mismo.
pub const GUST_DIR: &str = ".gust";

/// # Layout
/// Calcula la ruta de cada componente de `.gust/` a partir de la raíz del directorio de trabajo.
///
/// Solo construye rutas: no comprueba que existan ni toca el disco, salvo [`Layout::init`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    /// Raíz del directorio de trabajo (la carpeta que contiene `.gust/`)
    root: PathBuf,
}

impl Layout {
    /// Crea el layout de un repositorio cuya raíz es `root`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        todo!()
    }

    /// Crea `.gust/` con `objects/`, `refs/` y `logs/`, y un `HEAD` que apunta a
    /// `initial_branch` (que aún no existe).
    ///
    /// Falla con `io::ErrorKind::AlreadyExists` si `.gust/` ya existe, para no pisar un
    /// repositorio previo.
    pub fn init(&self, initial_branch: &str) -> io::Result<()> {
        todo!()
    }

    /// Raíz del directorio de trabajo.
    pub fn root(&self) -> &Path {
        todo!()
    }

    /// `<raíz>/.gust`
    pub fn gust_dir(&self) -> PathBuf {
        todo!()
    }

    /// `<raíz>/.gust/HEAD`
    pub fn head_path(&self) -> PathBuf {
        todo!()
    }

    /// `<raíz>/.gust/objects`
    pub fn objects_dir(&self) -> PathBuf {
        todo!()
    }

    /// Ruta del objeto `id`: `objects/` seguido de `id` en hexadecimal (en minúsculas).
    pub fn object_path(&self, id: &[u8]) -> PathBuf {
        todo!()
    }

    /// `<raíz>/.gust/refs`
    pub fn refs_dir(&self) -> PathBuf {
        todo!()
    }

    /// Ruta de la referencia `name`, donde cada `'/'` del nombre es una subcarpeta.
    ///
    /// Asume que `name` ya pasó por [`validate_ref_name`]: así no puede contener `".."` ni
    /// componentes vacíos que escapen de `refs/`.
    ///
    /// [`validate_ref_name`]: crate::refs::validate_ref_name
    pub fn ref_path(&self, name: &str) -> PathBuf {
        todo!()
    }

    /// `<raíz>/.gust/logs/refs`, el registro de cambios de referencias.
    pub fn refs_log_path(&self) -> PathBuf {
        todo!()
    }
}

/// Reemplaza el contenido de `path` por `bytes` de una sola vez: quien lea el archivo verá el
/// contenido anterior o el nuevo, nunca uno escrito a medias.
///
/// Lo hace en tres pasos:
/// 1. Escribe `bytes` en un archivo temporal dentro de la misma carpeta que `path`. Tiene que
///    ser la misma carpeta para que ambos queden en el mismo sistema de archivos, que es lo que
///    permite el `rename` instantáneo del paso 3.
/// 2. Llama a [`File::sync_all`] sobre el temporal. Al escribir, el sistema operativo suele
///    dejar los datos en memoria y bajarlos al disco más tarde; `sync_all` espera a que estén
///    guardados de verdad. Sin este paso, un corte de luz justo después del `rename` podría
///    dejar `path` vacío o incompleto.
/// 3. Renombra el temporal sobre `path` con [`fs::rename`], reemplazándolo de una vez.
///
/// [`File::sync_all`]: std::fs::File::sync_all
/// [`fs::rename`]: std::fs::rename
///
/// Si algo falla antes del paso 3, borra el temporal y `path` queda intacto. La carpeta padre
/// de `path` debe existir, porque esta función no la crea.
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    todo!()
}
