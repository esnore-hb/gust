//! Abstrae el almacenamiento del *estado mutable* del repositorio, es decir, está pensado
//! para el manejo de `HEAD` y referencias con nombre como ramas, tags, remotos y stash.
//!
//! A diferencia de [`crate::store2::ObjectStore`], que guarda objetos inmutables indexados por su
//! contenido, `RefStore` administra los punteros humanos (como ramas) que avanzan con cada *commit*.
//!
//! ## Repaso
//! ### Referencias de Git
//! Una **referencia** es un nombre que apunta al *hash* de un *commit*. Como los *commits* son
//! inmutables, las referencias son lo único que se mueve en el historial.
//! - **Rama:** avanza al nuevo *commit* cada vez que se hace un *commit* sobre ella.
//! - **Tag:** marca un *commit* fijo (ej. `v1.0`).
//! - **`HEAD`:** apunta al nombre de la rama actual, para saber cuál avanzar. Si apunta directo
//!   a un *commit*, está en modo *detached*. En un repositorio vacío, `HEAD` apunta a `main`
//!   aunque `main` aún no exista.
//!
//! ```text
//! HEAD ─▶ main ─▶ c3 ─▶ c2 ─▶ c1
//!                       ▲
//!                v1.0 ──┘
//! ```
//!
//! ### *Compare & swap*
//! Escritura condicional: "cambia la referencia a `new` solo si aún vale `expected`". Evita
//! que dos escrituras simultáneas se pisen (condición de carrera): la segunda falla con un
//! conflicto en vez de borrar el cambio de la primera.
//!
//! ### Registro de cambios
//! Cada mutación recibe una razón (ej. `"commit: inicial"`) que el *backend* puede registrar,
//! para saber después cuándo y por qué se movió una referencia (similar al *reflog* de Git).
//!
//! ## Diseño
//! Define el trait `RefStore` que busca el mismo diseño de [`HashStrategy`] y [`ObjectStore`]: basado en *estrategias*,
//! junto con eso la actualización (mutación) de una referencia exige que sea por medio de *compare & swap*, modelo propuesto
//! en el blog [git-at-any-scale](https://cursor.com/en-US/blog/git-at-any-scale#continuity).
//! 
//! Cada implementación define operaciones básicas de mutación y consulta en torno a `HEAD` y las diferentes referencias:
//! 
//! **Consulta**
//! - `get_head`: Consulta el `HEAD` actual, e.j. si devolviera `Head::Branch(`main`)` indicaría que se encuentra en la rama main
//! - `get_ref(name)`: Devuelve el hash al que apunta la referencia `name`. e.j. Si ya sabemos que estamos en `main`, esto devolvería
//!   hacia que `commit` apunta `main`, la rama actual. Si la referencia (e.j. rama) no existiera, obtendríamos `None`
//! - `get_refs`: Devuelve todas las referencias existentes y los commits a los que apuntan.
//!
//! **Mutación**
//!
//! Todas las mutaciones requieren de un parámetro `reason` para ser registradas por el *backend*, pues,
//! las operaciones de mutación son *ciegas* a las operaciones específicas que se realizan
//!
//! - `set_head` Cambia hacia adonde apunta `HEAD` y proporciona una razón para ser loggeada
//! - `compare_and_swap`: *Muta* el hash de una referencia. Se incluye el argumento `expected`
//!   que ayuda a los *backends* a implementar mutación sin condiciones de carrera, sin este,
//!   no habría forma de saber si las escrituras se están *pisando* entre sí.
//! - `advance_head`: Caso particular de `compare_and_swap`: Avanza la referencia asociada a `HEAD`
//!
//! ## Limitaciones conocidas
//! - **Sin modo *detached*:** [`Head`] solo contempla apuntar a una rama (`Head::Branch`).
//!   Se marca `#[non_exhaustive]` para extenderlo a futuro (si una variante nueva requiriese
//!   guardar `H::Digest` directo en `Head`, implicaría parametrizar el tipo o modelarlo en un
//!   sub-trait sin alterar `RefStore`).
//! - **Actualización de una sola referencia por llamada:** `compare_and_swap` es atómico sobre una
//!   única referencia a la vez; aún no agrupa cambios de múltiples referencias en una transacción.
//! - **Listado completo en memoria:** `list_refs` devuelve todas las referencias en un `BTreeMap`
//!   sin paginación ni filtrado por prefijo, suficiente para el MVP pero costoso con miles de ramas.

use std::{collections::BTreeMap, error::Error, fmt, io};

use crate::hash::HashStrategy;

/// # Head
/// Representa la referencia activa donde está posicionado el área de trabajo,
/// puede ser una rama o eventualmente un commit (estado dettached, no implementado).
///
/// Se marca `#[non_exhaustive]` para permitir agregar nuevos estados de `HEAD`
/// manteniendo el módulo abierto a extensión.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Head {
    /// Apunta simbólicamente al nombre de una rama (tenga o no *commits* registrados)
    Branch(String),
}

/// # RefError
/// Error producido al consultar o actualizar referencias en un [`RefStore`].
#[derive(Debug)]
#[non_exhaustive]
pub enum RefError {
    /// Error de entrada/salida del *backend* subyacente
    Io(io::Error),
    /// El nombre de la referencia es vacío o contiene caracteres inválidos
    InvalidName(&'static str),
    /// El valor actual de la referencia no coincide con el esperado en `compare_and_swap`
    Conflict,
    /// Los datos almacenados de las referencias tienen un formato inválido
    Corrupt(&'static str),
}

impl fmt::Display for RefError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "error de E/S en referencias: {err}"),
            Self::InvalidName(msg) => write!(f, "nombre de referencia inválido: {msg}"),
            Self::Conflict => write!(f, "conflicto de actualización concurrente en referencia"),
            Self::Corrupt(msg) => write!(f, "almacén de referencias corrupto: {msg}"),
        }
    }
}

impl Error for RefError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::InvalidName(_) | Self::Conflict | Self::Corrupt(_) => None,
        }
    }
}

impl From<io::Error> for RefError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

/// Valida que `name` sea un nombre de referencia seguro tanto para registros de texto
/// como para rutas jerárquicas en disco.
///
/// Rechaza nombres vacíos, con caracteres de control o espacios, que empiecen o terminen en `'/'`,
/// o que contengan componentes vacíos (`"//"`), `"."` o `".."`.
pub fn validate_ref_name(name: &str) -> Result<(), RefError> {
    if name.is_empty() {
        return Err(RefError::InvalidName("el nombre no puede ser vacío"));
    }
    if name.starts_with('/') || name.ends_with('/') {
        return Err(RefError::InvalidName(
            "el nombre no puede iniciar ni terminar con '/'",
        ));
    }
    if name.bytes().any(|b| b <= b' ' || b == 0x7f) { // caracteres de control
        return Err(RefError::InvalidName(
            "el nombre contiene espacios o caracteres de control",
        ));
    }
    if name
        .split('/')
        .any(|part| matches!(part, "" | "." | ".."))
    {
        return Err(RefError::InvalidName(
            "el nombre contiene un componente de ruta inválido",
        ));
    }
    Ok(())
}

/// # RefStore
/// Define la estrategia para consultar y actualizar las referencias mutables del repositorio.
///
/// Desacopla el manejo de ramas y `HEAD` del medio concreto de persistencia (memoria, archivos
/// sueltos o un registro *append-only* tipo WAL), permitiendo sumar nuevos *backends* o traits
/// de extensión sin modificar este módulo.
///
/// ## Ejemplo
/// ```ignore
/// // En un repositorio recién creado, HEAD apunta a "main" pero aún no hay commits
/// let parent = refs.resolve_head()?; // Ok(None)
///
/// // Tras guardar el primer Commit en ObjectStore, avanzamos la rama activa de forma atómica
/// refs.advance_head(parent.as_ref(), first_commit_id, "commit: inicial")?;
/// ```
pub trait RefStore<H: HashStrategy> {
    /// Devuelve el destino actual de `HEAD`.
    fn get_head(&self) -> Result<Head, RefError>;

    /// Cambia hacia donde apunta `HEAD`, registrando `reason` si el *backend* guarda *logs*.
    /// e.j. cambio de rama o entrar en modo dettached.
    /// ## Args
    /// * `&mut self`: muta el Store
    /// * `target`: Nuevo `Head::Branch(name)` u otros.
    /// * `reason`: descripción breve para el *log*.
    fn set_head(&mut self, target: Head, reason: &str) -> Result<(), RefError>;

    /// Devuelve el *digest* al que apunta la referencia `name`, o `Ok(None)` si aún no existe.
    fn get_ref(&self, name: &str) -> Result<Option<H::Digest>, RefError>;

    /// Devuelve todas las referencias existentes ordenadas alfabéticamente por nombre.
    fn list_refs(&self) -> Result<BTreeMap<String, H::Digest>, RefError>;

    /// Actualiza `name` a `new` (o la elimina si `new` es `None`) solo si su valor actual es `expected`.
    ///
    /// Falla con [`RefError::Conflict`] si la referencia cambió respecto a `expected`, o con
    /// [`RefError::InvalidName`] si `name` no cumple [`validate_ref_name`].
    /// 
    /// ## Args
    /// * `&mut self` - Muta el Store
    /// * `name` - Nombre de la referencia a actualizar.
    /// * `expected` - Estado esperado de la referencia `name`: 
    ///   - `None` si se espera que la referencia **exista** (evita sobreescribir sobre algo ya existente)
    ///   - `Some(old_id):` si se espera que apunte a `old_id` (evita pisar la escritura de otro)
    /// * `new` Estado final deseado:
    ///   - `Some(new_id)`: crea o actualiza `name` para que apunte a `new_id`
    ///   - `None`: Elimina la referencia (verificando antes con `expected` donde estaba parada)
    /// * `reason`: descripción breve para el *log*.
    fn compare_and_swap(
        &mut self,
        name: &str,
        expected: Option<&H::Digest>,
        new: Option<H::Digest>,
        reason: &str,
    ) -> Result<(), RefError>;

    /// Resuelve el *digest* del *commit* actual al que apunta `HEAD`, o `Ok(None)` si la rama aún no existe.
    fn resolve_head(&self) -> Result<Option<H::Digest>, RefError> {
        match self.get_head()? {
            Head::Branch(name) => self.get_ref(&name),
        }
    }

    /// Avanza la referencia activa en `HEAD` hacia `new_id`, verificando que antes estuviera en `expected`.
    /// ## Parámetros
    /// * `&mut self` - 
    /// * `expected` - Estado esperado de la referencia, e.j. commit al que apuntaba `HEAD`
    /// * `new_id`: nuevo id de `HEAD` para poder avanzarlo
    /// * `reason`: descripción breve para el *log*.
    fn advance_head(
        &mut self,
        expected: Option<&H::Digest>,
        new_id: H::Digest,
        reason: &str,
    ) -> Result<(), RefError> {
        match self.get_head()? {
            Head::Branch(name) => self.compare_and_swap(&name, expected, Some(new_id), reason),
        }
    }
}