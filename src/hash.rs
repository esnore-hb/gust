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

    /// Devuelve el id del objeto: el *hash* de [`GitObject::encode`].
    fn id(&self) -> <Self::H as HashStrategy>::Digest {
        Self::H::hash(&self.encode())
    }
}
