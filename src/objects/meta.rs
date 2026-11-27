

use std::{collections::BTreeMap};

use crate::hash::{Deserializable, DeserializeError, Serializable};

/// # Meta
/// Agrupa metadatos flexibles en pares clave-valor para describir un [`Commit`]
/// (ej. `"author"`, `"committer"` o `"date"`).
///
/// Usa un [`BTreeMap`] para mantener las claves **ordenadas** alfabéticamente,
/// lo que garantiza siempre la misma serialización de los mismos metadatos.
pub struct Meta {
    /// Diccionario clave valor: `{<data>:<description>}`
    pub fields: BTreeMap<String, String>,
}

impl Serializable for BTreeMap<String, String> {
    /// Formato: una línea `<key> <value>\n` por cada par, en orden alfabético de clave.
    fn serialize(&self) -> Vec<u8> {
        let total_len: usize = self.iter().map(|(k, v)| k.len() + v.len() + 2).sum();
        let mut buff = Vec::with_capacity(total_len);

        for (key, value) in self {
            buff.extend_from_slice(key.as_bytes());
            buff.push(b' ');
            buff.extend_from_slice(value.as_bytes());
            buff.push(b'\n');
        }

        buff
    }
}

impl Deserializable for BTreeMap<String, String> {
    /// Reconstruye el mapa a partir de líneas `<key> <value>\n`.
    ///
    /// Una secuencia vacía produce un mapa vacío.
    ///
    /// Rechaza claves desordenadas o repetidas: una clave repetida reemplazaría en silencio
    /// el valor anterior y la serialización resultante ya no coincidiría con los bytes leídos.
    fn deserialize(bytes: &[u8]) -> Result<Self, DeserializeError> {
        let mut rest = std::str::from_utf8(bytes)?;
        let mut map = BTreeMap::new();

        while !rest.is_empty() {
            let (line, tail) = rest
                .split_once('\n')
                .ok_or(DeserializeError::UnexpectedEof)?;
            let (key, value) = line.split_once(' ').ok_or(DeserializeError::InvalidFormat(
                "línea de metadato sin separador",
            ))?;
            // Forma canónica: claves estrictamente crecientes (sin desorden ni duplicados)
            if map
                .last_key_value()
                .is_some_and(|(prev, _): (&String, _)| prev.as_str() >= key)
            {
                return Err(DeserializeError::InvalidFormat(
                    "claves de metadatos desordenadas o duplicadas",
                ));
            }
            map.insert(key.to_owned(), value.to_owned());
            rest = tail;
        }

        Ok(map)
    }
}

impl Serializable for Meta {
    /// Formato: el mismo que su [`BTreeMap`], `<key> <value>\n` por cada campo.
    fn serialize(&self) -> Vec<u8> {
        self.fields.serialize()
    }
}

impl Deserializable for Meta {
    /// Reconstruye [`Meta`] usando el mismo formato `<key> <value>\n` de su [`BTreeMap`].
    fn deserialize(bytes: &[u8]) -> Result<Self, DeserializeError> {
        Ok(Self {
            fields: BTreeMap::deserialize(bytes)?,
        })
    }
}