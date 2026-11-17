# Gust

An user-friendly Git terminal written in Rust

## How to run

En la raíz del proyecto

```bash
cargo build
```

Y para ejecutar el binario, desde la raiz del proyecto

```bash
./target/debug/gust
```

El cual va a crear una carpeta de meta datos del programa. En el archivo
`main.rs` vas a ver que ahora está analizando los mismos archivos del
repositorio, excepto a la carpeta en si misma, y el `target` que genera Rust.
