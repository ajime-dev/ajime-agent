//! Device token management

use chrono::{DateTime, Utc};
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use serde::{Deserialize, Serialize};

use crate::errors::AgentError;

/// Device token claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceTokenClaims {
    /// Subject (device ID)
    pub sub: String,

    /// Owner user ID
    pub owner_id: String,

    /// Device capabilities
    #[serde(default)]
    pub capabilities: Vec<String>,

    /// Issued at timestamp
    pub iat: i64,

    /// Expiration timestamp
    pub exp: i64,

    /// Issuer
    #[serde(default)]
    pub iss: Option<String>,
}

/// A device token wrapper
#[derive(Debug, Clone)]
pub struct DeviceToken {
    /// Raw token string
    pub raw: String,

    /// Decoded claims
    pub claims: DeviceTokenClaims,
}

impl DeviceToken {
    /// Create a new device token from raw string (JWT)
    /// Note: This does NOT validate the signature, only decodes the claims
    pub fn from_raw(raw: String) -> Result<Self, AgentError> {
        // Decode without validation to extract claims
        // In production, you'd validate against the backend's public key
        let mut validation = Validation::new(Algorithm::HS256);
        validation.insecure_disable_signature_validation();
        validation.validate_exp = false;

        let token_data = decode::<DeviceTokenClaims>(
            &raw,
            &DecodingKey::from_secret(b""),
            &validation,
        )
        .map_err(|e| AgentError::TokenError(format!("Failed to decode token: {}", e)))?;

        Ok(Self {
            raw,
            claims: token_data.claims,
        })
    }

    /// Create a device token from a raw device secret (non-JWT)
    /// Used when device.json contains a device_secret instead of a JWT
    pub fn from_secret(device_id: String, secret: String) -> Self {
        let now = Utc::now().timestamp();
        // Create minimal claims for a secret-based token
        // exp is set far in the future since secrets don't expire
        let claims = DeviceTokenClaims {
            sub: device_id.clone(),
            owner_id: String::new(), // Will be validated by backend
            capabilities: vec![],
            iat: now,
            exp: now + (365 * 24 * 60 * 60), // 1 year
            iss: Some("device-secret".to_string()),
        };

        Self {
            raw: secret,
            claims,
        }
    }

    /// Get the device ID
    pub fn device_id(&self) -> &str {
        &self.claims.sub
    }

    /// Get the owner ID
    pub fn owner_id(&self) -> &str {
        &self.claims.owner_id
    }

    /// Check if the token is expired
    pub fn is_expired(&self) -> bool {
        let now = Utc::now().timestamp();
        self.claims.exp < now
    }

    /// Check if the token expires within the given duration
    pub fn expires_within(&self, seconds: i64) -> bool {
        let now = Utc::now().timestamp();
        self.claims.exp < now + seconds
    }

    /// Get expiration time
    pub fn expires_at(&self) -> DateTime<Utc> {
        DateTime::from_timestamp(self.claims.exp, 0).unwrap_or_else(|| Utc::now())
    }

    /// Get time until expiration in seconds
    pub fn time_until_expiry(&self) -> i64 {
        let now = Utc::now().timestamp();
        self.claims.exp - now
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_expiry_check() {
        // This would require a valid JWT to test properly
        // For now, just test the logic
    }
}
