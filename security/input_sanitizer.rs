//! Input Sanitization and Validation
//!
//! Comprehensive input validation and sanitization for:
//! - Payment amounts and currencies
//! - Account identifiers (BIC, IBAN, SWIFT)
//! - String fields (names, addresses, messages)
//! - Email addresses and phone numbers
//! - SQL injection prevention
//! - XSS prevention
//! - Command injection prevention

use regex::Regex;
use rust_decimal::Decimal;
use std::collections::HashSet;
use thiserror::Error;
use unicode_normalization::UnicodeNormalization;

/// Sanitization errors
#[derive(Error, Debug)]
pub enum SanitizationError {
    #[error("Invalid input: {0}")]
    Invalid(String),

    #[error("Input too long: max {max}, got {actual}")]
    TooLong { max: usize, actual: usize },

    #[error("Input too short: min {min}, got {actual}")]
    TooShort { min: usize, actual: usize },

    #[error("Invalid format: {0}")]
    InvalidFormat(String),

    #[error("Forbidden character: {0}")]
    ForbiddenCharacter(char),

    #[error("SQL injection attempt detected")]
    SqlInjection,

    #[error("XSS attempt detected")]
    XssAttempt,

    #[error("Command injection attempt detected")]
    CommandInjection,
}

pub type Result<T> = std::result::Result<T, SanitizationError>;

/// Input sanitizer
pub struct InputSanitizer {
    /// Regex for BIC codes
    bic_regex: Regex,

    /// Regex for IBAN
    iban_regex: Regex,

    /// Regex for email
    email_regex: Regex,

    /// Regex for phone numbers
    phone_regex: Regex,

    /// SQL injection patterns
    sql_patterns: Vec<Regex>,

    /// XSS patterns
    xss_patterns: Vec<Regex>,

    /// Command injection patterns
    command_patterns: Vec<Regex>,

    /// Forbidden characters
    forbidden_chars: HashSet<char>,
}

impl InputSanitizer {
    /// Create new input sanitizer
    pub fn new() -> Self {
        let bic_regex = Regex::new(r"^[A-Z]{6}[A-Z0-9]{2}([A-Z0-9]{3})?$").unwrap();
        let iban_regex = Regex::new(r"^[A-Z]{2}[0-9]{2}[A-Z0-9]+$").unwrap();
        let email_regex = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
        let phone_regex = Regex::new(r"^\+?[1-9]\d{1,14}$").unwrap();

        let sql_patterns = vec![
            Regex::new(r"(?i)(union|select|insert|update|delete|drop|create|alter|exec|execute|script|javascript|<script)").unwrap(),
            Regex::new(r"('|\"|;|--|/\*|\*/|xp_|sp_)").unwrap(),
        ];

        let xss_patterns = vec![
            Regex::new(r"(?i)(<script|<iframe|<object|<embed|<img|javascript:|onerror=|onload=)").unwrap(),
        ];

        let command_patterns = vec![
            Regex::new(r"(;|\||&|`|\$\(|\${|>|<)").unwrap(),
        ];

        let mut forbidden_chars = HashSet::new();
        forbidden_chars.insert('\0'); // Null byte
        forbidden_chars.insert('\x01'); // Control characters
        forbidden_chars.insert('\x02');
        forbidden_chars.insert('\x03');
        forbidden_chars.insert('\x04');
        forbidden_chars.insert('\x05');
        forbidden_chars.insert('\x06');
        forbidden_chars.insert('\x07');
        forbidden_chars.insert('\x08');
        forbidden_chars.insert('\x0b');
        forbidden_chars.insert('\x0c');
        forbidden_chars.insert('\x0e');
        forbidden_chars.insert('\x0f');

        Self {
            bic_regex,
            iban_regex,
            email_regex,
            phone_regex,
            sql_patterns,
            xss_patterns,
            command_patterns,
            forbidden_chars,
        }
    }

    /// Sanitize string (remove forbidden characters, normalize Unicode)
    pub fn sanitize_string(&self, input: &str, max_length: usize) -> Result<String> {
        if input.len() > max_length {
            return Err(SanitizationError::TooLong {
                max: max_length,
                actual: input.len(),
            });
        }

        // Check for forbidden characters
        for ch in input.chars() {
            if self.forbidden_chars.contains(&ch) {
                return Err(SanitizationError::ForbiddenCharacter(ch));
            }
        }

        // Normalize Unicode (NFC normalization)
        let normalized: String = input.nfc().collect();

        // Trim whitespace
        Ok(normalized.trim().to_string())
    }

    /// Validate and sanitize BIC code
    pub fn sanitize_bic(&self, bic: &str) -> Result<String> {
        let bic = bic.trim().to_uppercase();

        if !self.bic_regex.is_match(&bic) {
            return Err(SanitizationError::InvalidFormat(
                "Invalid BIC format".to_string(),
            ));
        }

        Ok(bic)
    }

    /// Validate and sanitize IBAN
    pub fn sanitize_iban(&self, iban: &str) -> Result<String> {
        let iban: String = iban.chars().filter(|c| !c.is_whitespace()).collect();
        let iban = iban.to_uppercase();

        if !self.iban_regex.is_match(&iban) {
            return Err(SanitizationError::InvalidFormat(
                "Invalid IBAN format".to_string(),
            ));
        }

        // IBAN length validation
        if iban.len() < 15 || iban.len() > 34 {
            return Err(SanitizationError::InvalidFormat(
                "IBAN length must be 15-34 characters".to_string(),
            ));
        }

        Ok(iban)
    }

    /// Validate and sanitize email
    pub fn sanitize_email(&self, email: &str) -> Result<String> {
        let email = email.trim().to_lowercase();

        if !self.email_regex.is_match(&email) {
            return Err(SanitizationError::InvalidFormat(
                "Invalid email format".to_string(),
            ));
        }

        if email.len() > 254 {
            return Err(SanitizationError::TooLong {
                max: 254,
                actual: email.len(),
            });
        }

        Ok(email)
    }

    /// Validate and sanitize phone number
    pub fn sanitize_phone(&self, phone: &str) -> Result<String> {
        let phone: String = phone.chars().filter(|c| c.is_numeric() || *c == '+').collect();

        if !self.phone_regex.is_match(&phone) {
            return Err(SanitizationError::InvalidFormat(
                "Invalid phone format (E.164)".to_string(),
            ));
        }

        Ok(phone)
    }

    /// Validate payment amount
    pub fn sanitize_amount(&self, amount: Decimal, min: Decimal, max: Decimal) -> Result<Decimal> {
        if amount <= Decimal::ZERO {
            return Err(SanitizationError::Invalid(
                "Amount must be positive".to_string(),
            ));
        }

        if amount < min {
            return Err(SanitizationError::Invalid(format!(
                "Amount below minimum: {}",
                min
            )));
        }

        if amount > max {
            return Err(SanitizationError::Invalid(format!(
                "Amount exceeds maximum: {}",
                max
            )));
        }

        // Check decimal places (max 2)
        if amount.scale() > 2 {
            return Err(SanitizationError::InvalidFormat(
                "Amount cannot have more than 2 decimal places".to_string(),
            ));
        }

        Ok(amount)
    }

    /// Check for SQL injection
    pub fn check_sql_injection(&self, input: &str) -> Result<()> {
        for pattern in &self.sql_patterns {
            if pattern.is_match(input) {
                return Err(SanitizationError::SqlInjection);
            }
        }

        Ok(())
    }

    /// Check for XSS
    pub fn check_xss(&self, input: &str) -> Result<()> {
        for pattern in &self.xss_patterns {
            if pattern.is_match(input) {
                return Err(SanitizationError::XssAttempt);
            }
        }

        Ok(())
    }

    /// Check for command injection
    pub fn check_command_injection(&self, input: &str) -> Result<()> {
        for pattern in &self.command_patterns {
            if pattern.is_match(input) {
                return Err(SanitizationError::CommandInjection);
            }
        }

        Ok(())
    }

    /// Sanitize payment reference (alphanumeric + limited special chars)
    pub fn sanitize_payment_reference(&self, reference: &str) -> Result<String> {
        let reference = self.sanitize_string(reference, 35)?; // ISO 20022 max

        // Check for injection attempts
        self.check_sql_injection(&reference)?;
        self.check_xss(&reference)?;
        self.check_command_injection(&reference)?;

        // Only allow alphanumeric, spaces, and limited punctuation
        let allowed_chars: HashSet<char> = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 .-/(),"
            .chars()
            .collect();

        for ch in reference.chars() {
            if !allowed_chars.contains(&ch) {
                return Err(SanitizationError::ForbiddenCharacter(ch));
            }
        }

        Ok(reference)
    }

    /// Sanitize name (person or company)
    pub fn sanitize_name(&self, name: &str) -> Result<String> {
        let name = self.sanitize_string(name, 140)?; // ISO 20022 max

        if name.is_empty() {
            return Err(SanitizationError::TooShort {
                min: 1,
                actual: 0,
            });
        }

        // Check for injection attempts
        self.check_sql_injection(&name)?;
        self.check_xss(&name)?;

        // Only allow letters, spaces, and common name characters
        let allowed_chars: HashSet<char> = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz '-.,áéíóúàèìòùäëïöüâêîôûãõñçÁÉÍÓÚÀÈÌÒÙÄËÏÖÜÂÊÎÔÛÃÕÑÇ"
            .chars()
            .collect();

        for ch in name.chars() {
            if !allowed_chars.contains(&ch) {
                return Err(SanitizationError::ForbiddenCharacter(ch));
            }
        }

        Ok(name)
    }

    /// Sanitize address
    pub fn sanitize_address(&self, address: &str) -> Result<String> {
        let address = self.sanitize_string(address, 70)?; // ISO 20022 max per line

        if address.is_empty() {
            return Err(SanitizationError::TooShort {
                min: 1,
                actual: 0,
            });
        }

        // Check for injection attempts
        self.check_sql_injection(&address)?;
        self.check_xss(&address)?;

        Ok(address)
    }

    /// Sanitize currency code
    pub fn sanitize_currency(&self, currency: &str) -> Result<String> {
        let currency = currency.trim().to_uppercase();

        if currency.len() != 3 {
            return Err(SanitizationError::InvalidFormat(
                "Currency code must be 3 characters".to_string(),
            ));
        }

        // Must be alphabetic
        if !currency.chars().all(|c| c.is_ascii_alphabetic()) {
            return Err(SanitizationError::InvalidFormat(
                "Currency code must be alphabetic".to_string(),
            ));
        }

        // Validate against ISO 4217 (basic check)
        let valid_currencies: HashSet<&str> = [
            "USD", "EUR", "GBP", "JPY", "CHF", "CAD", "AUD", "NZD", "SEK", "NOK", "DKK", "PLN",
            "CZK", "HUF", "RUB", "TRY", "ZAR", "BRL", "MXN", "CNY", "INR", "KRW", "SGD", "HKD",
            "THB", "MYR", "IDR", "PHP", "AED", "SAR", "QAR", "KWD", "BHD", "OMR", "JOD", "ILS",
            "EGP", "MAD", "NGN", "KES", "GHS",
        ]
        .iter()
        .copied()
        .collect();

        if !valid_currencies.contains(currency.as_str()) {
            return Err(SanitizationError::InvalidFormat(format!(
                "Unsupported currency: {}",
                currency
            )));
        }

        Ok(currency)
    }
}

impl Default for InputSanitizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_string() {
        let sanitizer = InputSanitizer::new();

        // Valid string
        let result = sanitizer.sanitize_string("Hello World", 100);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello World");

        // Too long
        let result = sanitizer.sanitize_string("Hello", 3);
        assert!(result.is_err());

        // With forbidden character
        let result = sanitizer.sanitize_string("Hello\0World", 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_sanitize_bic() {
        let sanitizer = InputSanitizer::new();

        // Valid BIC (8 characters)
        assert!(sanitizer.sanitize_bic("DEUTDEFF").is_ok());

        // Valid BIC (11 characters)
        assert!(sanitizer.sanitize_bic("DEUTDEFF500").is_ok());

        // Invalid BIC
        assert!(sanitizer.sanitize_bic("INVALID").is_err());
        assert!(sanitizer.sanitize_bic("12345678").is_err());
    }

    #[test]
    fn test_sanitize_iban() {
        let sanitizer = InputSanitizer::new();

        // Valid IBAN
        assert!(sanitizer.sanitize_iban("DE89 3704 0044 0532 0130 00").is_ok());

        // Invalid IBAN
        assert!(sanitizer.sanitize_iban("INVALID").is_err());
        assert!(sanitizer.sanitize_iban("DE").is_err());
    }

    #[test]
    fn test_sanitize_email() {
        let sanitizer = InputSanitizer::new();

        // Valid email
        assert!(sanitizer.sanitize_email("user@example.com").is_ok());

        // Invalid email
        assert!(sanitizer.sanitize_email("invalid").is_err());
        assert!(sanitizer.sanitize_email("@example.com").is_err());
    }

    #[test]
    fn test_sanitize_amount() {
        let sanitizer = InputSanitizer::new();

        let min = Decimal::new(1, 2); // 0.01
        let max = Decimal::new(100000000, 2); // 1,000,000.00

        // Valid amount
        assert!(sanitizer
            .sanitize_amount(Decimal::new(10000, 2), min, max)
            .is_ok());

        // Zero amount
        assert!(sanitizer
            .sanitize_amount(Decimal::ZERO, min, max)
            .is_err());

        // Below minimum
        assert!(sanitizer
            .sanitize_amount(Decimal::new(1, 3), min, max)
            .is_err());

        // Above maximum
        assert!(sanitizer
            .sanitize_amount(Decimal::new(200000000, 2), min, max)
            .is_err());
    }

    #[test]
    fn test_sql_injection() {
        let sanitizer = InputSanitizer::new();

        // SQL injection attempts
        assert!(sanitizer.check_sql_injection("' OR '1'='1").is_err());
        assert!(sanitizer.check_sql_injection("'; DROP TABLE users--").is_err());
        assert!(sanitizer.check_sql_injection("UNION SELECT * FROM").is_err());

        // Clean input
        assert!(sanitizer.check_sql_injection("Hello World").is_ok());
    }

    #[test]
    fn test_xss() {
        let sanitizer = InputSanitizer::new();

        // XSS attempts
        assert!(sanitizer.check_xss("<script>alert('XSS')</script>").is_err());
        assert!(sanitizer.check_xss("<img src=x onerror=alert(1)>").is_err());

        // Clean input
        assert!(sanitizer.check_xss("Hello World").is_ok());
    }

    #[test]
    fn test_command_injection() {
        let sanitizer = InputSanitizer::new();

        // Command injection attempts
        assert!(sanitizer.check_command_injection("file.txt; rm -rf /").is_err());
        assert!(sanitizer.check_command_injection("$(whoami)").is_err());
        assert!(sanitizer.check_command_injection("file.txt | cat").is_err());

        // Clean input
        assert!(sanitizer.check_command_injection("filename.txt").is_ok());
    }

    #[test]
    fn test_sanitize_name() {
        let sanitizer = InputSanitizer::new();

        // Valid names
        assert!(sanitizer.sanitize_name("John Smith").is_ok());
        assert!(sanitizer.sanitize_name("María García").is_ok());
        assert!(sanitizer.sanitize_name("O'Brien").is_ok());

        // Invalid names
        assert!(sanitizer.sanitize_name("<script>").is_err());
        assert!(sanitizer.sanitize_name("John123").is_err());
    }

    #[test]
    fn test_sanitize_currency() {
        let sanitizer = InputSanitizer::new();

        // Valid currencies
        assert!(sanitizer.sanitize_currency("USD").is_ok());
        assert!(sanitizer.sanitize_currency("EUR").is_ok());
        assert!(sanitizer.sanitize_currency("gbp").is_ok()); // Lowercase

        // Invalid currencies
        assert!(sanitizer.sanitize_currency("US").is_err()); // Too short
        assert!(sanitizer.sanitize_currency("USDD").is_err()); // Too long
        assert!(sanitizer.sanitize_currency("XXX").is_err()); // Not supported
    }
}