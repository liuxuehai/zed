use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::DockPosition;

/// Panel state for persistence between sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelState {
    /// Panel position in the workspace
    pub position: DockPosition,
    /// Panel size (width or height depending on position)
    pub size: f32,
    /// Whether the panel is visible
    pub visible: bool,
    /// Panel-specific state data
    pub custom_state: HashMap<String, serde_json::Value>,
}

impl Default for PanelState {
    fn default() -> Self {
        Self {
            position: DockPosition::Left,
            size: 300.0,
            visible: true,
            custom_state: HashMap::new(),
        }
    }
}

/// Manages persistence of panel states across sessions
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PanelPersistence {
    /// Map of panel names to their states
    pub panel_states: HashMap<String, PanelState>,
}

impl PanelPersistence {
    /// Create new panel persistence manager
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Load panel states from disk with proper error handling (.rules compliance)
    pub fn load(config_dir: &PathBuf) -> Result<Self> {
        let file_path = Self::get_persistence_file_path(config_dir)?;
        
        // Check if file exists with bounds checking
        if !file_path.exists() {
            return Ok(Self::default());
        }
        
        // Read file with proper error propagation
        let contents = fs::read_to_string(&file_path)?;
        let persistence: PanelPersistence = serde_json::from_str(&contents)?;
        
        Ok(persistence)
    }
    
    /// Save panel states to disk with proper error handling (.rules compliance)
    pub fn save(&self, config_dir: &PathBuf) -> Result<()> {
        let file_path = Self::get_persistence_file_path(config_dir)?;
        
        // Ensure parent directory exists
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        // Serialize and write with proper error propagation
        let contents = serde_json::to_string_pretty(self)?;
        fs::write(&file_path, contents)?;
        
        Ok(())
    }
    
    /// Get panel state with bounds checking (.rules compliance)
    pub fn get_panel_state(&self, panel_name: &str) -> Option<&PanelState> {
        self.panel_states.get(panel_name)
    }
    
    /// Set panel state with validation
    pub fn set_panel_state(&mut self, panel_name: String, state: PanelState) -> Result<()> {
        if panel_name.is_empty() {
            return Err(anyhow::anyhow!("Panel name cannot be empty"));
        }
        
        // Validate panel size
        if state.size <= 0.0 {
            return Err(anyhow::anyhow!("Panel size must be positive"));
        }
        
        self.panel_states.insert(panel_name, state);
        Ok(())
    }
    
    /// Update panel position with validation
    pub fn update_panel_position(&mut self, panel_name: &str, position: DockPosition) -> Result<()> {
        if let Some(state) = self.panel_states.get_mut(panel_name) {
            state.position = position;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Panel '{}' not found", panel_name))
        }
    }
    
    /// Update panel size with validation
    pub fn update_panel_size(&mut self, panel_name: &str, size: f32) -> Result<()> {
        if size <= 0.0 {
            return Err(anyhow::anyhow!("Panel size must be positive"));
        }
        
        if let Some(state) = self.panel_states.get_mut(panel_name) {
            state.size = size;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Panel '{}' not found", panel_name))
        }
    }
    
    /// Update panel visibility
    pub fn update_panel_visibility(&mut self, panel_name: &str, visible: bool) -> Result<()> {
        if let Some(state) = self.panel_states.get_mut(panel_name) {
            state.visible = visible;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Panel '{}' not found", panel_name))
        }
    }
    
    /// Set custom state data for a panel
    pub fn set_custom_state(&mut self, panel_name: &str, key: String, value: serde_json::Value) -> Result<()> {
        if let Some(state) = self.panel_states.get_mut(panel_name) {
            state.custom_state.insert(key, value);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Panel '{}' not found", panel_name))
        }
    }
    
    /// Get custom state data for a panel with bounds checking (.rules compliance)
    pub fn get_custom_state(&self, panel_name: &str, key: &str) -> Option<&serde_json::Value> {
        self.panel_states
            .get(panel_name)
            .and_then(|state| state.custom_state.get(key))
    }
    
    /// Remove panel state
    pub fn remove_panel(&mut self, panel_name: &str) {
        self.panel_states.remove(panel_name);
    }
    
    /// Clear all panel states
    pub fn clear(&mut self) {
        self.panel_states.clear();
    }
    
    /// Get persistence file path with proper error handling (.rules compliance)
    fn get_persistence_file_path(config_dir: &PathBuf) -> Result<PathBuf> {
        if !config_dir.exists() {
            fs::create_dir_all(config_dir)?;
        }
        
        Ok(config_dir.join("stock_trading_panels.json"))
    }
}

/// Theme colors for trading UI with proper validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingThemeColors {
    /// Color for positive price changes
    pub positive_color: String,
    /// Color for negative price changes
    pub negative_color: String,
    /// Color for neutral/unchanged prices
    pub neutral_color: String,
    /// Chart background color
    pub chart_background: Option<String>,
    /// Grid line color
    pub grid_color: String,
}

impl Default for TradingThemeColors {
    fn default() -> Self {
        Self {
            positive_color: "#00ff00".to_string(),
            negative_color: "#ff0000".to_string(),
            neutral_color: "#808080".to_string(),
            chart_background: None,
            grid_color: "#404040".to_string(),
        }
    }
}

impl TradingThemeColors {
    /// Validate color format (basic hex color validation)
    pub fn validate_color(color: &str) -> Result<()> {
        if !color.starts_with('#') || (color.len() != 7 && color.len() != 9) {
            return Err(anyhow::anyhow!("Invalid color format: {}", color));
        }
        
        // Check if all characters after # are valid hex digits
        for ch in color.chars().skip(1) {
            if !ch.is_ascii_hexdigit() {
                return Err(anyhow::anyhow!("Invalid hex color: {}", color));
            }
        }
        
        Ok(())
    }
    
    /// Update positive color with validation
    pub fn set_positive_color(&mut self, color: String) -> Result<()> {
        Self::validate_color(&color)?;
        self.positive_color = color;
        Ok(())
    }
    
    /// Update negative color with validation
    pub fn set_negative_color(&mut self, color: String) -> Result<()> {
        Self::validate_color(&color)?;
        self.negative_color = color;
        Ok(())
    }
    
    /// Update neutral color with validation
    pub fn set_neutral_color(&mut self, color: String) -> Result<()> {
        Self::validate_color(&color)?;
        self.neutral_color = color;
        Ok(())
    }
    
    /// Update grid color with validation
    pub fn set_grid_color(&mut self, color: String) -> Result<()> {
        Self::validate_color(&color)?;
        self.grid_color = color;
        Ok(())
    }
    
    /// Update chart background color with validation
    pub fn set_chart_background(&mut self, color: Option<String>) -> Result<()> {
        if let Some(ref color_str) = color {
            Self::validate_color(color_str)?;
        }
        self.chart_background = color;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    
    #[test]
    fn test_panel_state_default() {
        let state = PanelState::default();
        assert_eq!(state.position, DockPosition::Left);
        assert_eq!(state.size, 300.0);
        assert!(state.visible);
        assert!(state.custom_state.is_empty());
    }
    
    #[test]
    fn test_panel_persistence_new() {
        let persistence = PanelPersistence::new();
        assert!(persistence.panel_states.is_empty());
    }
    
    #[test]
    fn test_set_panel_state() {
        let mut persistence = PanelPersistence::new();
        let state = PanelState {
            position: DockPosition::Right,
            size: 400.0,
            visible: true,
            custom_state: HashMap::new(),
        };
        
        assert!(persistence.set_panel_state("test_panel".to_string(), state).is_ok());
        assert!(persistence.get_panel_state("test_panel").is_some());
    }
    
    #[test]
    fn test_set_panel_state_validation() {
        let mut persistence = PanelPersistence::new();
        
        // Empty panel name should fail
        let state = PanelState::default();
        assert!(persistence.set_panel_state(String::new(), state.clone()).is_err());
        
        // Invalid size should fail
        let invalid_state = PanelState {
            size: -10.0,
            ..state
        };
        assert!(persistence.set_panel_state("test".to_string(), invalid_state).is_err());
    }
    
    #[test]
    fn test_update_panel_position() {
        let mut persistence = PanelPersistence::new();
        let state = PanelState::default();
        
        persistence.set_panel_state("test".to_string(), state).ok();
        assert!(persistence.update_panel_position("test", DockPosition::Bottom).is_ok());
        
        let updated_state = persistence.get_panel_state("test").unwrap();
        assert_eq!(updated_state.position, DockPosition::Bottom);
    }
    
    #[test]
    fn test_update_panel_size() {
        let mut persistence = PanelPersistence::new();
        let state = PanelState::default();
        
        persistence.set_panel_state("test".to_string(), state).ok();
        assert!(persistence.update_panel_size("test", 500.0).is_ok());
        
        // Invalid size should fail
        assert!(persistence.update_panel_size("test", -100.0).is_err());
    }
    
    #[test]
    fn test_custom_state() {
        let mut persistence = PanelPersistence::new();
        let state = PanelState::default();
        
        persistence.set_panel_state("test".to_string(), state).ok();
        
        let value = serde_json::json!({"key": "value"});
        assert!(persistence.set_custom_state("test", "custom_key".to_string(), value.clone()).is_ok());
        
        let retrieved = persistence.get_custom_state("test", "custom_key");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), &value);
    }
    
    #[test]
    fn test_save_and_load() {
        let temp_dir = env::temp_dir().join("stock_trading_test");
        fs::create_dir_all(&temp_dir).ok();
        
        let mut persistence = PanelPersistence::new();
        let state = PanelState {
            position: DockPosition::Right,
            size: 350.0,
            visible: false,
            custom_state: HashMap::new(),
        };
        
        persistence.set_panel_state("test_panel".to_string(), state).ok();
        
        // Save
        assert!(persistence.save(&temp_dir).is_ok());
        
        // Load
        let loaded = PanelPersistence::load(&temp_dir).unwrap();
        assert!(loaded.get_panel_state("test_panel").is_some());
        
        let loaded_state = loaded.get_panel_state("test_panel").unwrap();
        assert_eq!(loaded_state.position, DockPosition::Right);
        assert_eq!(loaded_state.size, 350.0);
        assert!(!loaded_state.visible);
        
        // Cleanup
        fs::remove_dir_all(&temp_dir).ok();
    }
    
    #[test]
    fn test_theme_colors_validation() {
        assert!(TradingThemeColors::validate_color("#ff0000").is_ok());
        assert!(TradingThemeColors::validate_color("#00ff00ff").is_ok()); // With alpha
        assert!(TradingThemeColors::validate_color("ff0000").is_err()); // Missing #
        assert!(TradingThemeColors::validate_color("#ff00").is_err()); // Too short
        assert!(TradingThemeColors::validate_color("#gggggg").is_err()); // Invalid hex
    }
    
    #[test]
    fn test_theme_colors_setters() {
        let mut colors = TradingThemeColors::default();
        
        assert!(colors.set_positive_color("#00ff00".to_string()).is_ok());
        assert!(colors.set_negative_color("#ff0000".to_string()).is_ok());
        assert!(colors.set_neutral_color("#808080".to_string()).is_ok());
        assert!(colors.set_grid_color("#404040".to_string()).is_ok());
        
        // Invalid colors should fail
        assert!(colors.set_positive_color("invalid".to_string()).is_err());
    }
}
