
use sha1::{Digest, Sha1};
use std::{env, fs};

struct Commit {
    hash: String,
    blob_path: String,
    description: String,
}

struct Tree {}

fn to_blob(file_path: &str) -> String {
    let contents = fs::read(file_path)
        .expect("Error reading file");

    let mut hasher = Sha1::new();
    hasher.update(&contents);

    let hash = hasher.finalize();
    let hash_string = hash.iter()
        .map(|byte| format!("{:02x}", byte))
        .collect::<String>();

    hash_string
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let (_query, file_path) = parse_config(&args);

    let hash = to_blob(file_path);

    println!("SHA-1: {}", hash);

    // Ahora puedes comparar el hash con otro String.
    let other_hash = String::from("00e9c04bd4c8534bea1a2f26a7882765cd06fd7e");

    if hash == other_hash {
        println!("Los hashes coinciden");
    }
}

fn parse_config(args: &[String]) -> (&str, &str) {
    let query = &args[1];
    let file_path = &args[2];

    (query, file_path)
}
