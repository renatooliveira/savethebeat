use crate::error::AppError;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Verify Slack request signature
///
/// Slack signs all requests with HMAC-SHA256 using the signing secret.
/// This prevents replay attacks and ensures requests come from Slack.
///
/// # Algorithm
/// 1. Concatenate version, timestamp, and request body: `v0:{timestamp}:{body}`
/// 2. Compute HMAC-SHA256 with signing secret
/// 3. Compare with provided signature
///
/// # Arguments
/// * `signing_secret` - Slack app signing secret
/// * `timestamp` - Request timestamp header (X-Slack-Request-Timestamp)
/// * `body` - Raw request body
/// * `signature` - Signature from header (X-Slack-Signature)
///
/// # Returns
/// Ok(()) if signature is valid, Err otherwise
///
/// # Errors
/// - `SignatureInvalid` if signature doesn't match
/// - `SignatureMissing` if headers are missing
/// - `SignatureExpired` if timestamp is too old (>5 minutes)
pub fn verify_slack_signature(
    signing_secret: &str,
    timestamp: &str,
    body: &[u8],
    signature: &str,
) -> Result<(), AppError> {
    // Check timestamp to prevent replay attacks (max 5 minutes old)
    let request_timestamp = timestamp
        .parse::<i64>()
        .map_err(|_| AppError::SignatureInvalid("Invalid timestamp format".to_string()))?;

    let current_timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    if (current_timestamp - request_timestamp).abs() > 60 * 5 {
        return Err(AppError::SignatureExpired(
            "Request timestamp too old".to_string(),
        ));
    }

    // Build signature base string: v0:{timestamp}:{body}
    let sig_basestring = format!("v0:{}:", timestamp);
    let mut basestring_bytes = sig_basestring.into_bytes();
    basestring_bytes.extend_from_slice(body);

    // Compute HMAC-SHA256
    let mut mac = HmacSha256::new_from_slice(signing_secret.as_bytes())
        .map_err(|e| AppError::SignatureInvalid(format!("Invalid key: {}", e)))?;
    mac.update(&basestring_bytes);
    let result = mac.finalize();
    let computed_signature = format!("v0={}", hex::encode(result.into_bytes()));

    // Constant-time comparison to prevent timing attacks
    if signature != computed_signature {
        tracing::warn!(
            provided = signature,
            computed = computed_signature,
            "Slack signature verification failed"
        );
        return Err(AppError::SignatureInvalid(
            "Signature does not match".to_string(),
        ));
    }

    tracing::debug!("Slack signature verified successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_valid_signature() {
        let secret = "test_secret";
        let current_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let timestamp = current_timestamp.to_string();
        let body = b"test body";

        // Compute expected signature
        let sig_basestring = format!("v0:{}:", timestamp);
        let mut basestring_bytes = sig_basestring.into_bytes();
        basestring_bytes.extend_from_slice(body);

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(&basestring_bytes);
        let result_bytes = mac.finalize();
        let signature = format!("v0={}", hex::encode(result_bytes.into_bytes()));

        let result = verify_slack_signature(secret, &timestamp, body, &signature);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_invalid_signature() {
        let secret = "test_secret";
        let current_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let timestamp = current_timestamp.to_string();
        let body = b"test body";
        let signature = "v0=invalid_signature_here";

        let result = verify_slack_signature(secret, &timestamp, body, signature);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::SignatureInvalid(_)));
    }

    #[test]
    fn test_verify_expired_timestamp() {
        let secret = "test_secret";
        let timestamp = "1000000000"; // Very old timestamp
        let body = b"test body";
        let signature = "v0=anything";

        let result = verify_slack_signature(secret, timestamp, body, signature);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::SignatureExpired(_)));
    }

    #[test]
    fn test_verify_invalid_timestamp_format() {
        let secret = "test_secret";
        let timestamp = "not_a_number";
        let body = b"test body";
        let signature = "v0=anything";

        let result = verify_slack_signature(secret, timestamp, body, signature);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::SignatureInvalid(_)));
    }
}
