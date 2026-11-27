//! Define una interfaz común para calcular un *hash*,
//! cómo se convierte un objeto en bytes (y de vuelta) y cómo un objeto obtiene su identificador.
//!
//! No depende de ningún otro módulo del crate, mientras
//! que [`crate::core`] (los objetos) y [`crate::store2`] (su almacenamiento) se apoyan en él.
//!
//! ## Repaso
//! ### ¿Qué es un *hash* (en este contexto)?
//! Función que resume bytes de cualquier tamaño en un *digest* de tamaño fijo (ej. 20 bytes en
//! SHA-1). Los mismos bytes dan siempre el mismo *digest*, y bytes distintos, en la práctica,
//! nunca coinciden: sirve como huella digital del contenido.
//!
//! ### ¿Qué es serializar?
//! Convertir una estructura en memoria en una secuencia de bytes (y deserializar, el camino
//! inverso). Como el id se calcula sobre esos bytes, debe ser **determinista**: el mismo objeto
//! produce siempre los mismos bytes.
//!
//! ### ¿Y la serialización "estilo Git"?
//! Antes de hashear, Git antepone al cuerpo un *header* con el tipo y el largo:
//! `<KIND> <len>\0<body>` (ej. `blob 4\0hola`). El *hash* de eso es el **id** del objeto. El
//! `KIND` evita que objetos de distinto tipo con el mismo cuerpo (ej. un `Blob` y un `Tree`
//! vacíos) compartan id, y `<len>` permite detectar datos truncados.
//!
//! ## Diseño
//! - [`HashStrategy`]: el algoritmo, recibido como genérico para no fijar uno concreto.
//! - [`Serializable`] / [`Deserializable`]: objeto ↔ cuerpo en bytes; deserializar puede
//!   fallar con [`DeserializeError`].
//! - [`GitObject`]: declara `H` y `KIND`, y obtiene gratis `encode` (agrega el *header*),
//!   `decode` (lo valida) e `id`.
//!
//! ```text
//! objeto --serialize--> body --encode--> <KIND> <len>\0body --H::hash--> id
//! ```
//!
//! ## Limitaciones conocidas (post-MVP)
//! - **Copias en memoria al serializar y codificar:** `Serializable::serialize` reserva un
//!   `Vec<u8>` nuevo y `GitObject::encode` reserva otro `Vec<u8>` adicional para anteponer el
//!   *header* `<KIND> <len>\0`. En objetos grandes (ej. `Blob`) esto triplica temporalmente el
//!   uso de RAM; a futuro conviene escribir sobre un buffer compartido (`write_into` / `io::Write`).
//! - **Hashing en un solo paso:** `HashStrategy::hash` exige un slice contiguo en memoria.
//!   Exponer un *hasher* incremental permitiría calcular `GitObject::id` en *streaming* sin
//!   concatenar antes el *header* y el cuerpo.

use std::{error::Error, fmt, io::Write, str::Utf8Error};

/// # HashStrategy
/// Define el algoritmo que se usará para calcular el hash (ej. SHA-1 o SHA-256).
///
/// Sirve para desacoplar las estructuras del proyecto del algoritmo específico,
/// permitiendo cambiar cómo se calcula el hash sin tocar el resto del código
pub trait HashStrategy {
    /// El tipo de salida (ej. [u8; 20] o [u8; 32])
    type Digest: AsRef<[u8]> + Eq + Clone + std::fmt::Debug;
    /// Largo fijo en bytes del *digest* producido por esta estrategia (ej. 20 para SHA-1, 32 para SHA-256)
    const DIGEST_LEN: usize;
    /// Función de hash
    /// Dado un slice devuelve su hash de tipo Digest
    fn hash(data: &[u8]) -> Self::Digest;
    /// Reconstruye un *digest* a partir de sus bytes crudos.
    ///
    /// Devuelve `None` si `bytes.len() != Self::DIGEST_LEN`.
    fn digest_from_bytes(bytes: &[u8]) -> Option<Self::Digest>;
}

/// # Serializable
/// Permite convertir una estructura en una secuencia de bytes.
pub trait Serializable {
    /// Transforma el objeto actual en un vector de bytes (`Vec<u8>`)
    fn serialize(&self) -> Vec<u8>;
}

// ---------- Manejo de errores de deserialización ----------
// Tipo de error común a todas las implementaciones de Deserializable, junto con
// las conversiones necesarias para usarlo con el operador `?`.

/// # DeserializeError
/// Error producido al reconstruir un objeto desde una secuencia de bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeserializeError {
    /// Faltan bytes antes de completar la estructura esperada
    UnexpectedEof,
    /// Los bytes no son UTF-8 válido en un campo de texto
    InvalidUtf8(Utf8Error),
    /// El contenido no respeta el formato esperado del objeto
    InvalidFormat(&'static str),
}

impl fmt::Display for DeserializeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedEof => write!(f, "fin de datos inesperado al deserializar"),
            Self::InvalidUtf8(err) => write!(f, "UTF-8 inválido: {err}"),
            Self::InvalidFormat(msg) => write!(f, "formato inválido: {msg}"),
        }
    }
}

impl Error for DeserializeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidUtf8(err) => Some(err),
            Self::UnexpectedEof | Self::InvalidFormat(_) => None,
        }
    }
}

impl From<Utf8Error> for DeserializeError {
    fn from(err: Utf8Error) -> Self {
        Self::InvalidUtf8(err)
    }
}

// ----------------------------------------------------------

/// # Deserializable
/// Permite reconstruir una estructura a partir de una secuencia de bytes.
pub trait Deserializable: Sized {
    /// Reconstruye el objeto desde `bytes` o devuelve un [`DeserializeError`] si el formato es inválido.
    fn deserialize(bytes: &[u8]) -> Result<Self, DeserializeError>;
}

/// # GitObject
/// Permite que un objeto genere su propio ID único al estilo de Git.
///
/// Para calcular el ID, concatena el tipo de objeto (`KIND`), su tamaño en bytes,
/// un carácter nulo (`\0`) y su contenido serializado, y luego aplica el hash.
///
/// ## Ejemplo
/// ```ignore
/// struct Blob { content: Vec<u8> }
///
/// impl Serializable for Blob {
///     fn serialize(&self) -> Vec<u8> {
///         self.content.clone()
///     }
/// }
///
/// impl GitObject for Blob {
///     type H = HashKetchum; // algoritmo de hash a utilizar
///     const KIND: &'static str = "blob";
/// }
/// let file = Blob { content: b"hola".to_vec() };
/// // Hashea "blob 4\0hola"
/// let id = file.id();
/// ```
pub trait GitObject: Serializable {
    /// Estrategia de hash que usará este objeto
    type H: HashStrategy;
    /// Nombre del tipo de objeto estilo Git (ej. `"blob"`, `"tree"`, `"commit"`)
    const KIND: &'static str;
    /// Serializa el objeto junto con su *header* (`<KIND> <len>\0<body>`).
    ///
    /// Devuelve los bytes de la serialización para hashearlos o darle otro uso.
    fn encode(&self) -> Vec<u8> {
        let body = self.serialize();
        let len = body.len();
        // Cantidad de dígitos decimales de `len` (el 0 también ocupa un dígito)
        let len_digits = len.checked_ilog10().unwrap_or(0) as usize + 1;

        let mut buf = Vec::with_capacity(Self::KIND.len() + 1 + len_digits + 1 + len);
        // `write!` escribe directo en el Vec, sin crear un String intermedio
        write!(buf, "{} {}\0", Self::KIND, len).expect("escribir en un Vec no falla");
        buf.extend_from_slice(&body);
        buf
    }

    /// Valida el *header* (`<KIND> <len>\0`) de `raw` y deserializa el cuerpo en `Self`.
    ///
    /// Es la operación inversa de [`GitObject::encode`].
    fn decode(raw: &[u8]) -> Result<Self, DeserializeError>
    where
        Self: Deserializable,
    {
        // busca \0
        let nul = raw
            .iter()
            .position(|&b| b == 0)
            .ok_or(DeserializeError::UnexpectedEof)?;
        // Validación del header
        let header = std::str::from_utf8(&raw[..nul])?;
        let (kind, len_str) = header
            .split_once(' ')
            .ok_or(DeserializeError::InvalidFormat(
                "header sin separador de espacio",
            ))?;

        if kind != Self::KIND {
            return Err(DeserializeError::InvalidFormat(
                "tipo de objeto inesperado",
            ));
        }

        let expected_len: usize = len_str
            .parse()
            .map_err(|_| DeserializeError::InvalidFormat("largo inválido en header"))?;
        let body = &raw[nul + 1..];
        if body.len() != expected_len {
            return Err(DeserializeError::InvalidFormat(
                "el largo del cuerpo no coincide con el header",
            ));
        }

        Self::deserialize(body)
    }

    /// Devuelve el id del objeto: el *hash* de [`GitObject::encode`].
    fn id(&self) -> <Self::H as HashStrategy>::Digest {
        Self::H::hash(&self.encode())
    }
}
