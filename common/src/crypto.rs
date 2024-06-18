use blake2::digest::{Update, VariableOutput};
use chacha20poly1305::{aead::Aead, KeyInit, XChaCha20Poly1305};
use ed25519_dalek::{ed25519::signature::SignerMut, Verifier};
use rand::RngCore;
use x25519_dalek::{x25519, X25519_BASEPOINT_BYTES};
use zeroize::Zeroize;

pub use ed25519_dalek::{Signature, SigningKey, VerifyingKey};

pub const ED25519_SECRET_KEY_SIZE: usize = ed25519_dalek::SECRET_KEY_LENGTH;
pub const X25519_PUBLIC_KEY_SIZE: usize = 32;
pub const X25519_PRIVATE_KEY_SIZE: usize = 32;
pub const XCHACHA20_POLY1305_NONCE_SIZE: usize = 24;
pub const XCHACHA20_POLY1305_KEY_SIZE: usize = 32;

#[derive(thiserror::Error, Debug)]
pub enum CryptoError {
    #[error("signature verification failed: {0}")]
    SignatureVerification(ed25519_dalek::SignatureError),
    #[error("cipher failed: {0}")]
    Cipher(chacha20poly1305::aead::Error),
}

impl From<ed25519_dalek::SignatureError> for CryptoError {
    fn from(e: ed25519_dalek::SignatureError) -> Self {
        CryptoError::SignatureVerification(e)
    }
}

impl From<chacha20poly1305::aead::Error> for CryptoError {
    fn from(e: chacha20poly1305::aead::Error) -> Self {
        CryptoError::Cipher(e)
    }
}

pub type CryptoResult<T> = Result<T, CryptoError>;
pub type KeyExchangePublicKey = [u8; X25519_PUBLIC_KEY_SIZE];
pub type KeyExchangePrivateKey = [u8; X25519_PRIVATE_KEY_SIZE];
pub type Nonce = [u8; XCHACHA20_POLY1305_NONCE_SIZE];

pub fn get_signing_key() -> SigningKey {
    SigningKey::generate(&mut rand::rngs::OsRng {})
}

pub fn get_signing_key_from(secret_key: &[u8; ed25519_dalek::SECRET_KEY_LENGTH]) -> SigningKey {
    SigningKey::from_bytes(secret_key)
}

pub fn generate_key_exchange_key_pair(
    signing_key: &mut SigningKey,
) -> (KeyExchangePublicKey, KeyExchangePrivateKey, Signature) {
    let mut rand_generator = rand::rngs::OsRng {};

    // Generate ephemeral keypair for key exchange
    let mut ephemeral_private_key = [0u8; X25519_PRIVATE_KEY_SIZE];
    rand_generator.fill_bytes(&mut ephemeral_private_key);
    let ephemeral_public_key = x25519(ephemeral_private_key, X25519_BASEPOINT_BYTES);

    // Sign ephemeral public key
    let emphemeral_public_key_signature = signing_key.sign(&ephemeral_public_key);

    (
        ephemeral_public_key,
        ephemeral_private_key,
        emphemeral_public_key_signature,
    )
}

pub fn verify_key_exchange_key_pair(
    verifying_key: &VerifyingKey,
    ephemeral_public_key: KeyExchangePublicKey,
    signature: Signature,
) -> CryptoResult<()> {
    Ok(verifying_key.verify(&ephemeral_public_key, &signature)?)
}

pub fn encrypt(
    encryption_ephemeral_public_key: KeyExchangePublicKey,
    plain_data: &[u8],
) -> CryptoResult<(KeyExchangePublicKey, Nonce, Vec<u8>)> {
    let mut rand_generator = rand::rngs::OsRng {};

    // Generate ephemeral keypair
    let mut ephemeral_private_key = [0u8; X25519_PRIVATE_KEY_SIZE];
    rand_generator.fill_bytes(&mut ephemeral_private_key);
    let decryption_ephemeral_public_key =
        x25519(ephemeral_private_key, x25519_dalek::X25519_BASEPOINT_BYTES);

    // Key exchange
    let mut shared_secret = x25519(ephemeral_private_key, encryption_ephemeral_public_key);

    // Generate nonce
    let mut nonce = [0u8; XCHACHA20_POLY1305_NONCE_SIZE];
    rand_generator.fill_bytes(&mut nonce);

    // Derive key
    let mut kdf = blake2::VarBlake2b::new_keyed(&shared_secret, XCHACHA20_POLY1305_KEY_SIZE);
    kdf.update(nonce);
    let mut key = kdf.finalize_boxed();

    // Encrypt data
    let cipher = XChaCha20Poly1305::new(key.as_ref().into());
    let encrypted_data = cipher.encrypt(&nonce.into(), plain_data)?;

    shared_secret.zeroize();
    key.zeroize();

    Ok((decryption_ephemeral_public_key, nonce, encrypted_data))
}

fn make_signature_buffer(
    additional_data: &[u8],
    encrypted_data: &[u8],
    decryption_ephemeral_public_key: KeyExchangePublicKey,
    nonce: Nonce,
) -> Vec<u8> {
    [
        additional_data,
        encrypted_data,
        &decryption_ephemeral_public_key,
        &nonce,
    ]
    .concat()
}

pub fn sign(
    signing_key: &mut SigningKey,
    additional_data: &[u8],
    decryption_ephemeral_public_key: KeyExchangePublicKey,
    encrypted_data: &[u8],
    nonce: Nonce,
) -> Signature {
    // Signature
    signing_key.sign(&make_signature_buffer(
        additional_data,
        encrypted_data,
        decryption_ephemeral_public_key,
        nonce,
    ))
}

pub fn verify(
    verifying_key: &VerifyingKey,
    signature: Signature,
    additional_data: &[u8],
    decryption_ephemeral_public_key: KeyExchangePublicKey,
    encrypted_data: &[u8],
    nonce: Nonce,
) -> CryptoResult<()> {
    // Verify signature
    Ok(verifying_key.verify(
        &make_signature_buffer(
            additional_data,
            encrypted_data,
            decryption_ephemeral_public_key,
            nonce,
        ),
        &signature,
    )?)
}

pub fn decrypt(
    encrypted_data: &[u8],
    ephemeral_public_key: [u8; X25519_PUBLIC_KEY_SIZE],
    ephemeral_private_key: [u8; X25519_PRIVATE_KEY_SIZE],
    nonce: [u8; XCHACHA20_POLY1305_NONCE_SIZE],
) -> CryptoResult<Vec<u8>> {
    // Key exchange
    let mut shared_secret = x25519(ephemeral_private_key, ephemeral_public_key);

    // Derive key
    let mut kdf = blake2::VarBlake2b::new_keyed(&shared_secret, XCHACHA20_POLY1305_KEY_SIZE);
    kdf.update(nonce);
    let mut key = kdf.finalize_boxed();

    // Decrypt
    let cipher = XChaCha20Poly1305::new(key.as_ref().into());
    let plain_data = cipher.decrypt(&nonce.into(), encrypted_data)?;

    shared_secret.zeroize();
    key.zeroize();

    Ok(plain_data)
}

#[cfg(test)]
mod tests {
    use base64::{prelude::BASE64_STANDARD, Engine as _};
    use rand::RngCore;

    #[test]
    fn test_signature() {
        let mut signing_key = super::get_signing_key();
        println!(
            "[+] Signing key: {:?}",
            BASE64_STANDARD.encode(signing_key.as_bytes())
        );
        println!(
            "[+] Verifying key: {:?}",
            BASE64_STANDARD.encode(signing_key.verifying_key().as_bytes())
        );
        let verifying_key = signing_key.verifying_key();

        let mut rand_generator = rand::rngs::OsRng {};
        let data_id = "0";
        let agent_id = "0";
        let mut decryption_ephemeral_public_key = [0u8; 32];
        let mut encrypted_data = vec![0u8; 32];
        let mut nonce = [0u8; 24];
        rand_generator.fill_bytes(&mut decryption_ephemeral_public_key);
        rand_generator.fill_bytes(&mut encrypted_data);
        rand_generator.fill_bytes(&mut nonce);

        let signature = super::sign(
            &mut signing_key,
            &[data_id.as_bytes(), agent_id.as_bytes()].concat(),
            decryption_ephemeral_public_key,
            &encrypted_data,
            nonce,
        );

        super::verify(
            &verifying_key,
            signature,
            &[data_id.as_bytes(), agent_id.as_bytes()].concat(),
            decryption_ephemeral_public_key,
            &encrypted_data,
            nonce,
        )
        .unwrap();
    }

    #[test]
    fn test_encryption() {
        let mut rand_generator = rand::rngs::OsRng {};
        let mut encryption_ephemeral_private_key = [0u8; 32];
        rand_generator.fill_bytes(&mut encryption_ephemeral_private_key);
        let encryption_ephemeral_public_key = super::x25519(
            encryption_ephemeral_private_key.clone(),
            x25519_dalek::X25519_BASEPOINT_BYTES,
        );

        let plain_data = b"Hello, world!".to_vec();
        let (decryption_ephemeral_public_key, nonce, encrypted_data) =
            super::encrypt(encryption_ephemeral_public_key, &plain_data).unwrap();

        let decrypted_data = super::decrypt(
            &encrypted_data,
            decryption_ephemeral_public_key,
            encryption_ephemeral_private_key,
            nonce,
        )
        .unwrap();

        assert_eq!(plain_data, decrypted_data);
    }

    #[test]
    fn test_end_to_end() {
        let mut signing_key_alice = super::get_signing_key();
        let alice_verifying_key = signing_key_alice.verifying_key();
        let mut signing_key_bob = super::get_signing_key();
        let bob_verifying_key = signing_key_bob.verifying_key();

        let plain_data = "Hello, world!";
        let data_id = "1";
        let agent_id = "1";

        // Bob generates ephemeral keypair for key exchange
        let (ephemeral_public_key, ephemeral_private_key, ephemeral_private_key_signature) =
            super::generate_key_exchange_key_pair(&mut signing_key_bob);

        // Alice encrypts data for Bob
        println!("[+] Alice encrypts data for Bob");
        println!("[+] Plain data: {plain_data}");
        println!(
            "[+] Plain data {}",
            BASE64_STANDARD.encode(plain_data.as_bytes())
        );
        super::verify_key_exchange_key_pair(
            &bob_verifying_key,
            ephemeral_public_key,
            ephemeral_private_key_signature,
        )
        .unwrap();
        let (key_exchange_public_key, nonce, encrypted_data) =
            super::encrypt(ephemeral_public_key, plain_data.as_bytes()).unwrap();
        println!(
            "[+] Encrypted data: {}",
            BASE64_STANDARD.encode(&encrypted_data)
        );
        let signature = super::sign(
            &mut signing_key_alice,
            &[data_id.as_bytes(), agent_id.as_bytes()].concat(),
            key_exchange_public_key,
            &encrypted_data,
            nonce,
        );

        // Bob decrypts data from Alice
        println!("[+] Bob decrypts data from Alice");
        super::verify(
            &alice_verifying_key,
            signature,
            &[data_id.as_bytes(), agent_id.as_bytes()].concat(),
            key_exchange_public_key,
            &encrypted_data,
            nonce,
        )
        .unwrap();
        let decrypted_data = super::decrypt(
            &encrypted_data,
            key_exchange_public_key,
            ephemeral_private_key,
            nonce,
        )
        .unwrap();
        println!(
            "[+] Decrypted data {}",
            BASE64_STANDARD.encode(&decrypted_data)
        );
        println!(
            "[+] Decrypted data: {}",
            std::str::from_utf8(&decrypted_data).unwrap()
        );
        assert_eq!(plain_data.as_bytes(), decrypted_data);
    }
}
