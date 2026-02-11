// Input validation and error recovery module following Zed patterns (.rules compliance)

use anyhow::{anyhow, Result};
use std::collections::HashSet;

/// Stock symbol validator with comprehensive validation rules
pub struct SymbolValidator {
    #[allow(dead_code)] // Reserved for future exchange validation
    valid_exchanges: HashSet<String>,
    min_symbol_length: usize,
    max_symbol_length: usize,
    allow_numbers: bool,
    allow_special_chars: bool,
}

impl SymbolValidator {
    /// Create new validator with default US stock market rules
    pub fn new() -> Self {
        let mut valid_exchanges = HashSet::new();
        valid_exchanges.insert("NYSE".to_string());
        valid_exchanges.insert("NASDAQ".to_string());
        valid_exchanges.insert("AMEX".to_string());
        
        Self {
            valid_exchanges,
            min_symbol_length: 1,
            max_symbol_length: 5,
            allow_numbers: true,
            allow_special_chars: false,
        }
    }
    
    /// Create validator with custom configuration
    pub fn with_config(
        min_length: usize,
        max_length: usize,
        allow_numbers: bool,
        allow_special_chars: bool,
    ) -> Self {
        Self {
            valid_exchanges: HashSet::new(),
            min_symbol_length: min_length,
            max_symbol_length: max_length,
            allow_numbers,
            allow_special_chars,
        }
    }
    
    /// Validate stock symbol with detailed error messages (.rules compliance)
    pub fn validate_symbol(&self, symbol: &str) -> Result<String> {
        // Trim whitespace
        let symbol = symbol.trim().to_uppercase();
        
        // Check if empty
        if symbol.is_empty() {
            return Err(anyhow!(
                "Stock symbol cannot be empty. Please enter a valid symbol (e.g., AAPL, GOOGL)."
            ));
        }
        
        // Check length with bounds checking (.rules compliance)
        if symbol.len() < self.min_symbol_length {
            return Err(anyhow!(
                "Symbol '{}' is too short. Minimum length is {} character(s).",
                symbol,
                self.min_symbol_length
            ));
        }
        
        if symbol.len() > self.max_symbol_length {
            return Err(anyhow!(
                "Symbol '{}' is too long. Maximum length is {} character(s). US stock symbols are typically 1-5 characters.",
                symbol,
                self.max_symbol_length
            ));
        }
        
        // Validate characters
        for (index, character) in symbol.chars().enumerate() {
            if !character.is_ascii_alphabetic() {
                if character.is_ascii_digit() && !self.allow_numbers {
                    return Err(anyhow!(
                        "Symbol '{}' contains a number at position {}. Numbers are not allowed in stock symbols.",
                        symbol,
                        index + 1
                    ));
                } else if !character.is_ascii_alphanumeric() && !self.allow_special_chars {
                    return Err(anyhow!(
                        "Symbol '{}' contains invalid character '{}' at position {}. Only letters and numbers are allowed.",
                        symbol,
                        character,
                        index + 1
                    ));
                }
            }
        }
        
        // Additional validation: first character should be a letter
        if let Some(first_char) = symbol.chars().next()
            && !first_char.is_ascii_alphabetic()
        {
            return Err(anyhow!(
                "Symbol '{}' must start with a letter, not '{}'.",
                symbol,
                first_char
            ));
        }
        
        Ok(symbol)
    }
    
    /// Suggest corrections for invalid symbols
    pub fn suggest_corrections(&self, symbol: &str) -> Vec<String> {
        let mut suggestions = Vec::new();
        
        // Remove whitespace
        let cleaned = symbol.trim().to_uppercase();
        if cleaned != symbol && !cleaned.is_empty() {
            suggestions.push(cleaned.clone());
        }
        
        // Remove special characters
        let alphanumeric: String = symbol.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
        if alphanumeric != symbol && !alphanumeric.is_empty() && alphanumeric.len() <= self.max_symbol_length {
            suggestions.push(alphanumeric.to_uppercase());
        }
        
        // Truncate if too long
        if symbol.len() > self.max_symbol_length {
            let truncated = symbol.chars().take(self.max_symbol_length).collect::<String>();
            if !truncated.is_empty() {
                suggestions.push(truncated.to_uppercase());
            }
        }
        
        suggestions
    }
}

impl Default for SymbolValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Order parameter validator for trading operations
pub struct OrderValidator {
    min_quantity: u64,
    max_quantity: u64,
    min_price: f64,
    max_price: f64,
    price_precision: u32, // Number of decimal places
}

impl OrderValidator {
    /// Create new order validator with default rules
    pub fn new() -> Self {
        Self {
            min_quantity: 1,
            max_quantity: 1_000_000,
            min_price: 0.01,
            max_price: 1_000_000.0,
            price_precision: 2,
        }
    }
    
    /// Validate order quantity with bounds checking (.rules compliance)
    pub fn validate_quantity(&self, quantity: u64) -> Result<u64> {
        if quantity < self.min_quantity {
            return Err(anyhow!(
                "Order quantity {} is below minimum of {}. Please enter at least {} share(s).",
                quantity,
                self.min_quantity,
                self.min_quantity
            ));
        }
        
        if quantity > self.max_quantity {
            return Err(anyhow!(
                "Order quantity {} exceeds maximum of {}. Please enter no more than {} shares.",
                quantity,
                self.max_quantity,
                self.max_quantity
            ));
        }
        
        Ok(quantity)
    }
    
    /// Validate order price with proper error handling (.rules compliance)
    pub fn validate_price(&self, price: f64) -> Result<f64> {
        if price.is_nan() || price.is_infinite() {
            return Err(anyhow!(
                "Invalid price value. Please enter a valid number."
            ));
        }
        
        if price < self.min_price {
            return Err(anyhow!(
                "Price ${:.2} is below minimum of ${:.2}. Please enter a price of at least ${:.2}.",
                price,
                self.min_price,
                self.min_price
            ));
        }
        
        if price > self.max_price {
            return Err(anyhow!(
                "Price ${:.2} exceeds maximum of ${:.2}. Please enter a price no greater than ${:.2}.",
                price,
                self.max_price,
                self.max_price
            ));
        }
        
        // Validate precision
        let multiplier = 10_f64.powi(self.price_precision as i32);
        let rounded = (price * multiplier).round() / multiplier;
        
        if (price - rounded).abs() > 0.0001 {
            return Err(anyhow!(
                "Price ${:.2} has too many decimal places. Maximum precision is {} decimal place(s). Suggested: ${:.2}",
                price,
                self.price_precision,
                rounded
            ));
        }
        
        Ok(rounded)
    }
    
    /// Validate limit order (price must be specified)
    pub fn validate_limit_order(&self, price: Option<f64>) -> Result<f64> {
        match price {
            Some(p) => self.validate_price(p),
            None => Err(anyhow!(
                "Limit orders require a price. Please specify a limit price."
            )),
        }
    }
    
    /// Validate market order (price should not be specified)
    pub fn validate_market_order(&self, price: Option<f64>) -> Result<()> {
        if price.is_some() {
            return Err(anyhow!(
                "Market orders do not accept a price parameter. The order will execute at the current market price."
            ));
        }
        Ok(())
    }
    
    /// Validate stop price for stop orders
    pub fn validate_stop_price(&self, stop_price: f64, current_price: f64, is_buy: bool) -> Result<f64> {
        let validated_price = self.validate_price(stop_price)?;
        
        // For buy stop orders, stop price should be above current price
        // For sell stop orders, stop price should be below current price
        if is_buy && validated_price <= current_price {
            return Err(anyhow!(
                "Buy stop price ${:.2} must be above current market price ${:.2}.",
                validated_price,
                current_price
            ));
        }
        
        if !is_buy && validated_price >= current_price {
            return Err(anyhow!(
                "Sell stop price ${:.2} must be below current market price ${:.2}.",
                validated_price,
                current_price
            ));
        }
        
        Ok(validated_price)
    }
    
    /// Calculate order value and validate against limits
    pub fn validate_order_value(&self, quantity: u64, price: f64, max_order_value: f64) -> Result<f64> {
        let validated_quantity = self.validate_quantity(quantity)?;
        let validated_price = self.validate_price(price)?;
        
        let order_value = (validated_quantity as f64) * validated_price;
        
        if order_value > max_order_value {
            return Err(anyhow!(
                "Order value ${:.2} exceeds maximum allowed value of ${:.2}. Please reduce quantity or price.",
                order_value,
                max_order_value
            ));
        }
        
        Ok(order_value)
    }
}

impl Default for OrderValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Input parser with error recovery for numeric inputs
pub struct InputParser;

impl InputParser {
    /// Parse quantity from string with error recovery (.rules compliance)
    pub fn parse_quantity(input: &str) -> Result<u64> {
        let trimmed = input.trim();
        
        if trimmed.is_empty() {
            return Err(anyhow!("Quantity cannot be empty. Please enter a number."));
        }
        
        // Remove common formatting characters
        let cleaned = trimmed.replace([',', ' '], "");
        
        match cleaned.parse::<u64>() {
            Ok(value) => Ok(value),
            Err(_) => {
                // Try to extract numbers from the string
                let numbers_only: String = cleaned.chars().filter(|c| c.is_ascii_digit()).collect();
                
                if numbers_only.is_empty() {
                    Err(anyhow!(
                        "Could not parse '{}' as a quantity. Please enter a valid whole number (e.g., 100).",
                        input
                    ))
                } else {
                    match numbers_only.parse::<u64>() {
                        Ok(value) => {
                            log::warn!("Recovered quantity {} from input '{}'", value, input);
                            Ok(value)
                        }
                        Err(_) => Err(anyhow!(
                            "Could not parse '{}' as a quantity. The number may be too large.",
                            input
                        )),
                    }
                }
            }
        }
    }
    
    /// Parse price from string with error recovery (.rules compliance)
    pub fn parse_price(input: &str) -> Result<f64> {
        let trimmed = input.trim();
        
        if trimmed.is_empty() {
            return Err(anyhow!("Price cannot be empty. Please enter a number."));
        }
        
        // Remove common formatting characters and currency symbols
        let cleaned = trimmed.replace([',', ' ', '$', '€', '£'], "");
        
        match cleaned.parse::<f64>() {
            Ok(value) => {
                if value.is_nan() || value.is_infinite() {
                    Err(anyhow!("Invalid price value. Please enter a valid number."))
                } else {
                    Ok(value)
                }
            }
            Err(_) => Err(anyhow!(
                "Could not parse '{}' as a price. Please enter a valid decimal number (e.g., 150.50).",
                input
            )),
        }
    }
    
    /// Parse percentage from string
    pub fn parse_percentage(input: &str) -> Result<f64> {
        let trimmed = input.trim();
        
        if trimmed.is_empty() {
            return Err(anyhow!("Percentage cannot be empty. Please enter a number."));
        }
        
        // Remove percentage sign if present
        let cleaned = trimmed.replace(['%', ' '], "");
        
        match cleaned.parse::<f64>() {
            Ok(value) => {
                if value.is_nan() || value.is_infinite() {
                    Err(anyhow!("Invalid percentage value. Please enter a valid number."))
                } else if !(0.0..=100.0).contains(&value) {
                    Err(anyhow!(
                        "Percentage {:.2}% is out of range. Please enter a value between 0 and 100.",
                        value
                    ))
                } else {
                    Ok(value)
                }
            }
            Err(_) => Err(anyhow!(
                "Could not parse '{}' as a percentage. Please enter a valid number (e.g., 5.5).",
                input
            )),
        }
    }
}

/// Error recovery strategies for different error types
pub enum RecoveryStrategy {
    RetryWithCachedData,
    RetryWithDefaultValue,
    PromptUserForCorrection,
    SkipOperation,
    FallbackToAlternative,
}

/// Error recovery manager
pub struct ErrorRecoveryManager {
    retry_count: u32,
    max_retries: u32,
}

impl ErrorRecoveryManager {
    /// Create new error recovery manager
    pub fn new(max_retries: u32) -> Self {
        Self {
            retry_count: 0,
            max_retries,
        }
    }
    
    /// Determine recovery strategy based on error type
    pub fn determine_strategy(&mut self, error: &anyhow::Error) -> RecoveryStrategy {
        let error_message = error.to_string().to_lowercase();
        
        // Check if we've exceeded retry limit
        if self.retry_count >= self.max_retries {
            return RecoveryStrategy::PromptUserForCorrection;
        }
        
        // Determine strategy based on error content
        if error_message.contains("network") || error_message.contains("connection") {
            self.retry_count += 1;
            RecoveryStrategy::RetryWithCachedData
        } else if error_message.contains("parse") || error_message.contains("invalid") {
            RecoveryStrategy::PromptUserForCorrection
        } else if error_message.contains("not found") || error_message.contains("unavailable") {
            RecoveryStrategy::FallbackToAlternative
        } else {
            self.retry_count += 1;
            RecoveryStrategy::RetryWithDefaultValue
        }
    }
    
    /// Reset retry counter
    pub fn reset(&mut self) {
        self.retry_count = 0;
    }
    
    /// Get current retry count
    pub fn get_retry_count(&self) -> u32 {
        self.retry_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_symbol_validation() {
        let validator = SymbolValidator::new();
        
        // Valid symbols
        assert!(validator.validate_symbol("AAPL").is_ok());
        assert!(validator.validate_symbol("GOOGL").is_ok());
        assert!(validator.validate_symbol("A").is_ok());
        
        // Invalid symbols
        assert!(validator.validate_symbol("").is_err());
        assert!(validator.validate_symbol("TOOLONG").is_err());
        assert!(validator.validate_symbol("123").is_err());
        assert!(validator.validate_symbol("AA$PL").is_err());
    }
    
    #[test]
    fn test_quantity_validation() {
        let validator = OrderValidator::new();
        
        // Valid quantities
        assert!(validator.validate_quantity(1).is_ok());
        assert!(validator.validate_quantity(100).is_ok());
        assert!(validator.validate_quantity(1000).is_ok());
        
        // Invalid quantities
        assert!(validator.validate_quantity(0).is_err());
        assert!(validator.validate_quantity(10_000_000).is_err());
    }
    
    #[test]
    fn test_price_validation() {
        let validator = OrderValidator::new();
        
        // Valid prices
        assert!(validator.validate_price(10.50).is_ok());
        assert!(validator.validate_price(0.01).is_ok());
        assert!(validator.validate_price(1000.00).is_ok());
        
        // Invalid prices
        assert!(validator.validate_price(0.0).is_err());
        assert!(validator.validate_price(-10.0).is_err());
        assert!(validator.validate_price(f64::NAN).is_err());
        assert!(validator.validate_price(f64::INFINITY).is_err());
    }
    
    #[test]
    fn test_input_parsing() {
        // Quantity parsing
        assert_eq!(InputParser::parse_quantity("100").unwrap(), 100);
        assert_eq!(InputParser::parse_quantity("1,000").unwrap(), 1000);
        assert_eq!(InputParser::parse_quantity(" 50 ").unwrap(), 50);
        
        // Price parsing
        assert_eq!(InputParser::parse_price("10.50").unwrap(), 10.50);
        assert_eq!(InputParser::parse_price("$100.00").unwrap(), 100.00);
        assert_eq!(InputParser::parse_price("1,234.56").unwrap(), 1234.56);
        
        // Error cases
        assert!(InputParser::parse_quantity("").is_err());
        assert!(InputParser::parse_quantity("abc").is_err());
        assert!(InputParser::parse_price("").is_err());
    }
    
    #[test]
    fn test_symbol_suggestions() {
        let validator = SymbolValidator::new();
        
        let suggestions = validator.suggest_corrections(" aapl ");
        assert!(suggestions.contains(&"AAPL".to_string()));
        
        let suggestions = validator.suggest_corrections("AA$PL");
        assert!(suggestions.iter().any(|s| !s.contains('$')));
    }
}
