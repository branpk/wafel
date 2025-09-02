use std::io;

use argon2::{Argon2, Params};
use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    ChaCha20Poly1305, Nonce,
};
use serde::{Deserialize, Serialize};

/// Pure Rust password-based encryption using Argon2 + ChaCha20Poly1305
#[derive(Serialize, Deserialize)]
pub struct PureBox {
    /// Salt for Argon2 key derivation
    salt: [u8; 32],
    /// Nonce for ChaCha20Poly1305
    nonce: [u8; 12],
    /// Encrypted data
    ciphertext: Vec<u8>,
}

impl PureBox {
    /// Create a new encrypted box, encrypting `data` with `password`
    pub fn seal(password: &[u8], data: &[u8]) -> Result<Self, CryptoError> {
        // Generate random salt and nonce
        let salt = rand::random::<[u8; 32]>();
        let nonce_bytes = ChaCha20Poly1305::generate_nonce(&mut OsRng);

        // Derive key from password using Argon2
        let key = derive_key(password, &salt)?;

        // Encrypt data
        let cipher = ChaCha20Poly1305::new_from_slice(&key)
            .map_err(|_| CryptoError::KeyDerivationError)?;
        
        let ciphertext = cipher
            .encrypt(&nonce_bytes, data)
            .map_err(|_| CryptoError::EncryptionError)?;

        Ok(PureBox {
            salt,
            nonce: nonce_bytes.into(),
            ciphertext,
        })
    }

    /// Decrypt the box using the password
    pub fn open(&self, password: &[u8]) -> Result<Vec<u8>, CryptoError> {
        // Derive key from password using the stored salt
        let key = derive_key(password, &self.salt)?;

        // Decrypt data
        let cipher = ChaCha20Poly1305::new_from_slice(&key)
            .map_err(|_| CryptoError::KeyDerivationError)?;
        
        let nonce = Nonce::from_slice(&self.nonce);
        let plaintext = cipher
            .decrypt(nonce, self.ciphertext.as_ref())
            .map_err(|_| CryptoError::DecryptionError)?;

        Ok(plaintext)
    }
}

/// Derive a 32-byte key from password and salt using Argon2
fn derive_key(password: &[u8], salt: &[u8; 32]) -> Result<[u8; 32], CryptoError> {
    let params = Params::new(65536, 3, 1, Some(32))
        .map_err(|_| CryptoError::KeyDerivationError)?;
    
    let argon2 = Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        params,
    );

    let mut key = [0u8; 32];
    argon2
        .hash_password_into(password, salt, &mut key)
        .map_err(|_| CryptoError::KeyDerivationError)?;

    Ok(key)
}

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("Key derivation failed")]
    KeyDerivationError,
    #[error("Encryption failed")]
    EncryptionError,
    #[error("Decryption failed")]
    DecryptionError,
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
}