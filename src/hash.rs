


/// # HashStrategy
/// Define el algoritmo que se usará para calcular el hash (ej. SHA-1 o SHA-256).
/// 
/// Sirve para desacoplar las estructuras del proyecto del algoritmo específico,
/// permitiendo cambiar cómo se calcula el hash sin tocar el resto del código
pub trait HashStrategy {
    /// El tipo de salida (ej. [u8; 20] o [u8; 32])
    type Digest: AsRef<[u8]> + Eq + Clone + std::fmt::Debug;
    /// Función de hash
    /// Dado un slice devuelve su hash de tipo Digest
    fn hash(data: &[u8]) -> Self::Digest;
}

/**
 * # Serializable
 * Permite convertir una estructura en una secuencia de bytes.
 */
pub trait Serializable {
    /// Transforma el objeto actual en un vector de bytes (`Vec<u8>`)
    fn serialize(&self) -> Vec<u8>;
}


/// # GitObject
/// Permite que un objeto genere su propio ID único al estilo de Git.
/// 
/// Para calcular el ID, concatena el tipo de objeto (`KIND`), su tamaño en bytes,
/// un carácter nulo (`\0`) y su contenido serializado, y luego aplica el hash.
/// 
/// ## Ejemplo
/// ```ignore
/// struct Blob { content: Vec<u8> };
/// 
/// impl Serializable for Blob {
///     fn serialize(&self) -> Vec<u8> {
///         self.content 
///     }
/// }
/// 
/// impl GitObject for Blob {
///     type H = HashKetchum // algoritmo de Hash a utilizar
///     const KIND: &'static str = "blob";
/// }
/// let archivo = Blob { content: b"hola".to_vec() };
/// // Hashea "blob 4\0hola"
/// let id = archivo.id();
/// ```
pub trait GitObject: Serializable {
    /// Estrategia de hash que usará este objeto
    type H: HashStrategy;
    /// Nombre del tipo de objeto estilo Git (ej. `"blob"`, `"tree"`, `"commit"`)
    const KIND: &'static str;
    /// Genera y retorna el id del objeto mezclando su *header* y sus datos serializados
    fn id(&self) -> <Self::H as HashStrategy>::Digest {
        let body = self.serialize();
        let mut buf = format!("{} {}\0", Self::KIND, body.len()).into_bytes(); 
        buf.extend_from_slice(&body);
        Self::H::hash(&buf)
    }
}