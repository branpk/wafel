use std::fs;
use tempfile::NamedTempFile;

use crate::pure_crypto::PureBox;

#[test]
fn test_pure_crypto_roundtrip() {
    let password = b"test_password_123";
    let original_data = b"This is some test data that should be encrypted and decrypted correctly!";

    // Test encryption and decryption
    let encrypted_box = PureBox::seal(password, original_data)
        .expect("Encryption should succeed");
    
    let decrypted_data = encrypted_box.open(password)
        .expect("Decryption should succeed");
    
    assert_eq!(original_data.as_slice(), decrypted_data.as_slice(), 
               "Decrypted data should match original data");
}

#[test]
fn test_pure_crypto_wrong_password() {
    let password = b"correct_password";
    let wrong_password = b"wrong_password";
    let data = b"test data";

    let encrypted_box = PureBox::seal(password, data)
        .expect("Encryption should succeed");
    
    let result = encrypted_box.open(wrong_password);
    assert!(result.is_err(), "Decryption with wrong password should fail");
}

#[test]
fn test_pure_crypto_serialization() {
    let password = b"test_password";
    let data = b"test data for serialization";

    let encrypted_box = PureBox::seal(password, data)
        .expect("Encryption should succeed");
    
    // Serialize to JSON
    let serialized = serde_json::to_vec(&encrypted_box)
        .expect("Serialization should succeed");
    
    // Deserialize from JSON
    let deserialized: PureBox = serde_json::from_slice(&serialized)
        .expect("Deserialization should succeed");
    
    // Decrypt and verify
    let decrypted = deserialized.open(password)
        .expect("Decryption should succeed");
    
    assert_eq!(data.as_slice(), decrypted.as_slice());
}

#[test]
fn test_lock_unlock_integration() {
    // Create temporary files
    let dll_file = NamedTempFile::new().expect("Failed to create temp DLL file");
    let rom_file = NamedTempFile::new().expect("Failed to create temp ROM file");
    let locked_file = NamedTempFile::new().expect("Failed to create temp locked file");
    let unlocked_file = NamedTempFile::new().expect("Failed to create temp unlocked file");

    // Write test data
    let dll_data = b"This is fake DLL data for testing";
    // ROM data needs proper header and be multiple of 4 bytes
    let mut rom_data = b"\x80\x37\x12\x40".to_vec(); // Proper N64 ROM header
    rom_data.extend_from_slice(b"This is fake ROM data for testing (rest of data)!!!!"); // Pad to multiple of 4
    fs::write(dll_file.path(), dll_data).expect("Failed to write DLL data");
    fs::write(rom_file.path(), &rom_data).expect("Failed to write ROM data");

    // Test locking
    crate::try_lock_libsm64(
        dll_file.path().to_str().unwrap(),
        locked_file.path().to_str().unwrap(),
        rom_file.path().to_str().unwrap(),
    ).expect("Locking should succeed");

    // Verify locked file exists and contains data
    let locked_data = fs::read(locked_file.path()).expect("Failed to read locked file");
    assert!(!locked_data.is_empty(), "Locked file should not be empty");
    assert_ne!(locked_data.as_slice(), dll_data.as_slice(), "Locked data should be different from original");

    // Test unlocking
    crate::try_unlock_libsm64(
        locked_file.path().to_str().unwrap(),
        unlocked_file.path().to_str().unwrap(),
        rom_file.path().to_str().unwrap(),
    ).expect("Unlocking should succeed");

    // Verify unlocked data matches original
    let unlocked_data = fs::read(unlocked_file.path()).expect("Failed to read unlocked file");
    assert_eq!(unlocked_data.as_slice(), dll_data.as_slice(), "Unlocked data should match original DLL data");
}

#[test]
fn test_lock_unlock_wrong_rom() {
    // Create temporary files
    let dll_file = NamedTempFile::new().expect("Failed to create temp DLL file");
    let rom_file = NamedTempFile::new().expect("Failed to create temp ROM file");
    let wrong_rom_file = NamedTempFile::new().expect("Failed to create temp wrong ROM file");
    let locked_file = NamedTempFile::new().expect("Failed to create temp locked file");
    let unlocked_file = NamedTempFile::new().expect("Failed to create temp unlocked file");

    // Write test data - must be at least 4 bytes and multiple of 4 for ROM format
    let dll_data = b"This is fake DLL data for testing";
    // ROM data needs proper header and be multiple of 4 bytes
    let mut rom_data = b"\x80\x37\x12\x40".to_vec(); // Proper N64 ROM header
    rom_data.extend_from_slice(b"This is fake ROM data for testing!!!"); // Pad to multiple of 4
    let mut wrong_rom_data = b"\x80\x37\x12\x40".to_vec(); // Same header format
    wrong_rom_data.extend_from_slice(b"This is different ROM data!!!!!!!!"); // Different content, pad to multiple of 4
    
    fs::write(dll_file.path(), dll_data).expect("Failed to write DLL data");
    fs::write(rom_file.path(), &rom_data).expect("Failed to write ROM data");
    fs::write(wrong_rom_file.path(), &wrong_rom_data).expect("Failed to write wrong ROM data");

    // Lock with original ROM
    crate::try_lock_libsm64(
        dll_file.path().to_str().unwrap(),
        locked_file.path().to_str().unwrap(),
        rom_file.path().to_str().unwrap(),
    ).expect("Locking should succeed");

    // Try to unlock with wrong ROM - should fail
    let result = crate::try_unlock_libsm64(
        locked_file.path().to_str().unwrap(),
        unlocked_file.path().to_str().unwrap(),
        wrong_rom_file.path().to_str().unwrap(),
    );
    
    assert!(result.is_err(), "Unlocking with wrong ROM should fail");
    
    // Try to unlock with correct ROM - should succeed
    crate::try_unlock_libsm64(
        locked_file.path().to_str().unwrap(),
        unlocked_file.path().to_str().unwrap(),
        rom_file.path().to_str().unwrap(),
    ).expect("Unlocking with correct ROM should succeed");

    let unlocked_data = fs::read(unlocked_file.path()).expect("Failed to read unlocked file");
    assert_eq!(unlocked_data.as_slice(), dll_data.as_slice(), "Unlocked data should match original DLL data");
}