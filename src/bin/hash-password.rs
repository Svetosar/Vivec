use std::io::{self, Read};

use argon2::{Argon2, password_hash::PasswordHasher};

fn main() {
    let mut password = String::new();
    if io::stdin().read_to_string(&mut password).is_err() {
        eprintln!("не удалось прочитать пароль из stdin");
        std::process::exit(1);
    }

    let password = password.trim_end_matches(['\r', '\n']);
    if password.is_empty() {
        eprintln!("пароль пустой");
        std::process::exit(1);
    }

    match Argon2::default().hash_password(password.as_bytes()) {
        Ok(hash) => println!("{hash}"),
        Err(error) => {
            eprintln!("не удалось создать хеш: {error}");
            std::process::exit(1);
        }
    }
}
