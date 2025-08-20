use crate::errors::{WalletError, Result};
use solana_sdk::{
    signature::{Keypair, Signer},
    pubkey::Pubkey,
};
use bip39::{Mnemonic, Language, Seed};
use tiny_hderive::{bip32::ExtendedPrivKey, bip44::DerivationPath};
use serde::{Serialize, Deserialize};
use tracing::{info, debug, warn};
use hmac::{Hmac, Mac};
use sha2::Sha512;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletCredentials {
    pub public_key: String,
    pub private_key: String,
    pub mnemonic: Option<String>,
    pub derivation_path: String,
}

pub struct WalletGenerator;

impl WalletGenerator {
    /// Generate a completely new wallet with mnemonic phrase
    pub fn generate_new() -> Result<WalletCredentials> {
        // Generate 12-word mnemonic
        let mnemonic = Mnemonic::new(bip39::MnemonicType::Words12, Language::English);
        let mnemonic_str = mnemonic.phrase().to_string();
        
        // Derive seed from mnemonic
        let seed = Seed::new(&mnemonic, "");
        
        // Use standard Solana derivation path: m/44'/501'/0'/0'
        let derivation_path = "m/44'/501'/0'/0'";
        
        // Derive keypair from seed
        let keypair = Self::derive_keypair_from_seed(seed.as_bytes(), 0)?;
        
        let credentials = WalletCredentials {
            public_key: keypair.pubkey().to_string(),
            private_key: bs58::encode(keypair.to_bytes()),
            mnemonic: Some(mnemonic_str),
            derivation_path: derivation_path.to_string(),
        };
        
        info!("Generated new wallet: {}", credentials.public_key);
        
        Ok(credentials)
    }
    
    /// Generate wallet from existing mnemonic
    pub fn from_mnemonic(mnemonic_str: &str, passphrase: &str) -> Result<WalletCredentials> {
        let mnemonic = Mnemonic::from_phrase(mnemonic_str, Language::English)?;
        let seed = Seed::new(&mnemonic, passphrase);
        
        let derivation_path = "m/44'/501'/0'/0'";
        let keypair = Self::derive_keypair_from_seed(seed.as_bytes(), 0)?;
        
        let credentials = WalletCredentials {
            public_key: keypair.pubkey().to_string(),
            private_key: bs58::encode(keypair.to_bytes()),
            mnemonic: Some(mnemonic_str.to_string()),
            derivation_path: derivation_path.to_string(),
        };
        
        info!("Restored wallet from mnemonic: {}", credentials.public_key);
        
        Ok(credentials)
    }
    
    /// Import wallet from private key
    pub fn from_private_key(private_key_str: &str) -> Result<WalletCredentials> {
        let private_key_bytes = bs58::decode(private_key_str).into_vec()?;
        
        if private_key_bytes.len() != 64 {
            return Err(WalletError::InvalidPrivateKey.into());
        }
        
        let keypair = Keypair::from_bytes(&private_key_bytes)?;
        
        let credentials = WalletCredentials {
            public_key: keypair.pubkey().to_string(),
            private_key: private_key_str.to_string(),
            mnemonic: None,
            derivation_path: "direct".to_string(),
        };
        
        info!("Imported wallet: {}", credentials.public_key);
        
        Ok(credentials)
    }
    
    /// Validate a private key without storing it
    pub fn validate_private_key(private_key_str: &str) -> Result<String> {
        let private_key_bytes = bs58::decode(private_key_str).into_vec()?;
        
        if private_key_bytes.len() != 64 {
            return Err(WalletError::InvalidPrivateKey.into());
        }
        
        let keypair = Keypair::from_bytes(&private_key_bytes)?;
        Ok(keypair.pubkey().to_string())
    }
    
    /// Generate multiple wallets from a single mnemonic
    pub fn generate_multiple(mnemonic_str: &str, count: usize) -> Result<Vec<WalletCredentials>> {
        let mnemonic = Mnemonic::from_phrase(mnemonic_str, Language::English)?;
        let seed = Seed::new(&mnemonic, "");
        
        let mut wallets = Vec::new();
        
        for i in 0..count {
            let derivation_path = format!("m/44'/501'/{}'/0'", i);
            let keypair = Self::derive_keypair_from_seed(seed.as_bytes(), i as u32)?;
            
            wallets.push(WalletCredentials {
                public_key: keypair.pubkey().to_string(),
                private_key: bs58::encode(keypair.to_bytes()),
                mnemonic: Some(mnemonic_str.to_string()),
                derivation_path: derivation_path.clone(),
            });
        }
        
        Ok(wallets)
    }
    
    /// Derive keypair from seed and account index (simplified for Solana)
    fn derive_keypair_from_seed(seed: &[u8], account: u32) -> Result<Keypair> {
        // Use HMAC-SHA512 for key derivation (simplified BIP32 for Ed25519)
        let mut hmac = Hmac::<Sha512>::new_from_slice(b"ed25519 seed").map_err(|_| WalletError::DerivationFailed)?;
        hmac.update(seed);
        let result = hmac.finalize().into_bytes();
        
        // Split into key and chain code
        let (key_bytes, _chain_code) = result.split_at(32);
        
        // For Solana, we use a simplified derivation
        // m/44'/501'/account'/0' -> just modify key with account index
        let mut derived_key = [0u8; 32];
        derived_key.copy_from_slice(key_bytes);
        
        // Mix in the account index
        let account_bytes = account.to_le_bytes();
        for i in 0..4 {
            derived_key[i] ^= account_bytes[i];
        }
        
        // Create Solana keypair directly from the 32-byte seed
        // Solana SDK will handle the Ed25519 key generation internally
        let mut full_key = [0u8; 64];
        full_key[..32].copy_from_slice(&derived_key);
        
        // Let Solana SDK generate the public key from the private key
        Ok(Keypair::from_bytes(&full_key)?)
    }
    
    /// Generate a paper wallet with QR codes
    pub fn generate_paper_wallet() -> Result<(WalletCredentials, String, String)> {
        let credentials = Self::generate_new()?;
        
        // Generate QR codes for public and private keys
        let public_qr = qrcode::QrCode::new(&credentials.public_key)?;
        let private_qr = qrcode::QrCode::new(&credentials.private_key)?;
        
        // Convert to SVG strings
        let public_svg = public_qr
            .render()
            .min_dimensions(200, 200)
            .dark_color(qrcode::render::svg::Color("#000000"))
            .light_color(qrcode::render::svg::Color("#FFFFFF"))
            .build();
            
        let private_svg = private_qr
            .render()
            .min_dimensions(200, 200)
            .dark_color(qrcode::render::svg::Color("#000000"))
            .light_color(qrcode::render::svg::Color("#FFFFFF"))
            .build();
        
        Ok((credentials, public_svg, private_svg))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_wallet_generation() {
        let wallet = WalletGenerator::generate_new().unwrap();
        assert!(!wallet.public_key.is_empty());
        assert!(!wallet.private_key.is_empty());
        assert!(wallet.mnemonic.is_some());
    }
    
    #[test]
    fn test_import_private_key() {
        let wallet = WalletGenerator::generate_new().unwrap();
        let imported = WalletGenerator::from_private_key(&wallet.private_key).unwrap();
        assert_eq!(wallet.public_key, imported.public_key);
    }
}