//! Key Generator CLI Tool
//! 
//! Usage: keygen --name "User Name"
//! 
//! This tool generates encrypted EXAD-prefixed keys for users.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::env;

// Include secrets at compile time
include!("../secrets.rs");

#[derive(serde::Serialize)]
struct KeyPayload {
    uid: String,
    name: String,
    created: i64,
}

fn generate_uid(name: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(name.as_bytes());
    hasher.update(Utc::now().timestamp().to_le_bytes());
    // Add some randomness
    hasher.update(rand::random::<[u8; 16]>());
    let result = hasher.finalize();
    format!("u_{}", hex::encode(&result[..8]))
}

fn encrypt_key(payload: &KeyPayload) -> Result<String, String> {
    let json = serde_json::to_string(payload).map_err(|e| e.to_string())?;
    
    // Generate a random nonce (12 bytes for AES-GCM)
    let nonce_bytes: [u8; 12] = rand::random();
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    let cipher = Aes256Gcm::new_from_slice(MASTER_ENCRYPTION_KEY)
        .map_err(|e| e.to_string())?;
    
    let ciphertext = cipher
        .encrypt(nonce, json.as_bytes())
        .map_err(|e| e.to_string())?;
    
    // Combine nonce + ciphertext and encode
    let mut combined = nonce_bytes.to_vec();
    combined.extend(ciphertext);
    
    let encoded = URL_SAFE_NO_PAD.encode(&combined);
    Ok(format!("EXAD-{}", encoded))
}

fn print_usage() {
    println!("Sync2Bucket Key Generator");
    println!();
    println!("Usage: keygen --name \"User Name\"");
    println!();
    println!("Options:");
    println!("  --name <name>    User's name (required)");
    println!("  --help           Show this help message");
    println!();
    println!("Example:");
    println!("  keygen --name \"John Doe\"");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 || args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        print_usage();
        return;
    }
    
    // Parse arguments
    let mut name: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--name" | "-n" => {
                if i + 1 < args.len() {
                    name = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: --name requires a value");
                    std::process::exit(1);
                }
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                std::process::exit(1);
            }
        }
    }
    
    let name = match name {
        Some(n) => n,
        None => {
            eprintln!("Error: --name is required");
            print_usage();
            std::process::exit(1);
        }
    };
    
    // Generate key
    let payload = KeyPayload {
        uid: generate_uid(&name),
        name: name.clone(),
        created: Utc::now().timestamp(),
    };
    
    match encrypt_key(&payload) {
        Ok(key) => {
            println!();
            println!("╔══════════════════════════════════════════════════════════════╗");
            println!("║                    SYNC2BUCKET LICENSE KEY                    ║");
            println!("╠══════════════════════════════════════════════════════════════╣");
            println!("║ User: {:<54} ║", name);
            println!("║ UID:  {:<54} ║", payload.uid);
            println!("║ Created: {:<51} ║", Utc::now().format("%Y-%m-%d %H:%M:%S UTC"));
            println!("╠══════════════════════════════════════════════════════════════╣");
            println!("║ Key:                                                         ║");
            println!("║                                                              ║");
            
            // Word wrap the key for display
            let key_display = &key;
            let chunk_size = 58;
            for chunk in key_display.as_bytes().chunks(chunk_size) {
                let s = std::str::from_utf8(chunk).unwrap_or("");
                println!("║ {:<60} ║", s);
            }
            
            println!("║                                                              ║");
            println!("╚══════════════════════════════════════════════════════════════╝");
            println!();
            println!("Full key (copy this):");
            println!("{}", key);
            println!();
        }
        Err(e) => {
            eprintln!("Error generating key: {}", e);
            std::process::exit(1);
        }
    }
}
