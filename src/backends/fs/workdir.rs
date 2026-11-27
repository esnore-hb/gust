use std::path::PathBuf;

use crate::workdir::{DirEntry, WorkDir, WorkDirError, WorkDirMut};

/// # FsWorkDir
/// Lee y escribe los archivos del usuario directamente en una carpeta del disco.
///
/// Traduce las rutas relativas con `'/'` de [`WorkDir`] a rutas del sistema operativo bajo
/// `root`, y oculta la carpeta [`GUST_DIR`](super::GUST_DIR) de la raíz.
#[derive(Debug, Clone)]
pub struct FsWorkDir {
    /// Raíz del directorio de trabajo (la misma de [`Layout::root`](super::Layout::root))
    root: PathBuf,
}

impl FsWorkDir {
    /// Abre el directorio de trabajo con raíz en `root`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        todo!()
    }

    /// Convierte una ruta de [`WorkDir`] (ej. `"src/main.rs"`, o `""` para la raíz) en una ruta
    /// del sistema bajo `root`, agregando un componente por cada segmento separado por `'/'`.
    fn to_fs_path(&self, path: &str) -> PathBuf {
        todo!()
    }
}

impl WorkDir for FsWorkDir {
    /// Lista `dir` con `fs::read_dir`, usando el `file_type()` de cada entrada (que no sigue
    /// *symlinks*) para quedarse solo con archivos y directorios.
    ///
    /// En la raíz omite [`GUST_DIR`](super::GUST_DIR). Un nombre que no es UTF-8 válido no se
    /// puede guardar en un [`Tree`](crate::objects::Tree), así que falla con
    /// [`WorkDirError::InvalidName`] en vez de omitirlo en silencio.
    fn read_dir(&self, dir: &str) -> Result<Vec<DirEntry>, WorkDirError> {
        todo!()
    }

    /// Lee el archivo completo con `fs::read`; traduce `io::ErrorKind::NotFound` a
    /// [`WorkDirError::NotFound`].
    fn read_file(&self, path: &str) -> Result<Vec<u8>, WorkDirError> {
        todo!()
    }
}

impl WorkDirMut for FsWorkDir {
    /// Crea las carpetas padres que falten y escribe `content` con
    /// [`write_atomic`](super::write_atomic), para no dejar archivos a medias si se interrumpe
    /// un *checkout*.
    fn write_file(&mut self, path: &str, content: &[u8]) -> Result<(), WorkDirError> {
        todo!()
    }

    /// Borra el archivo y luego sube por sus carpetas padres borrando las que quedaron vacías,
    /// sin pasar nunca de `root`.
    fn remove_file(&mut self, path: &str) -> Result<(), WorkDirError> {
        todo!()
    }
}
