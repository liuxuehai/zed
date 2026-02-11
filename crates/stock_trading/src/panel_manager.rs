use anyhow::Result;
use gpui::{App, AppContext, Context, Entity, EventEmitter, px, Pixels, Render, Subscription, WeakEntity, Window};
use std::collections::HashMap;
use util::ResultExt;

use crate::{
    DockPosition, PanelPersistence, PanelState, TradingManager, StockTradingSettings,
};

/// Panel layout configuration with proportional sizing
#[derive(Debug, Clone)]
pub struct PanelLayout {
    /// Panel position in workspace
    pub position: DockPosition,
    /// Proportional size (0.0 to 1.0 of available space)
    pub proportion: f32,
    /// Minimum size in pixels
    pub min_size: Pixels,
    /// Maximum size in pixels (None for unlimited)
    pub max_size: Option<Pixels>,
    /// Whether panel is currently visible
    pub visible: bool,
}

impl Default for PanelLayout {
    fn default() -> Self {
        Self {
            position: DockPosition::Left,
            proportion: 0.25,
            min_size: px(200.0),
            max_size: Some(px(600.0)),
            visible: true,
        }
    }
}

impl PanelLayout {
    /// Validate layout configuration with bounds checking (.rules compliance)
    pub fn validate(&self) -> Result<()> {
        if self.proportion <= 0.0 || self.proportion > 1.0 {
            return Err(anyhow::anyhow!(
                "Panel proportion must be between 0.0 and 1.0, got {}",
                self.proportion
            ));
        }
        
        let min_size_value = f32::from(self.min_size);
        if min_size_value <= 0.0 {
            return Err(anyhow::anyhow!(
                "Panel minimum size must be positive, got {}",
                min_size_value
            ));
        }
        
        if let Some(max_size) = self.max_size {
            let max_size_value = f32::from(max_size);
            if max_size_value < min_size_value {
                return Err(anyhow::anyhow!(
                    "Panel maximum size ({}) must be >= minimum size ({})",
                    max_size_value,
                    min_size_value
                ));
            }
        }
        
        Ok(())
    }
    
    /// Calculate actual size based on available space with bounds checking (.rules compliance)
    pub fn calculate_size(&self, available_space: Pixels) -> Pixels {
        let available_value = f32::from(available_space);
        let proportional_size = px(available_value * self.proportion);
        
        // Clamp to min/max bounds
        let size = proportional_size.max(self.min_size);
        
        if let Some(max_size) = self.max_size {
            size.min(max_size)
        } else {
            size
        }
    }
    
    /// Update proportion while maintaining constraints
    pub fn set_proportion(&mut self, proportion: f32) -> Result<()> {
        if proportion <= 0.0 || proportion > 1.0 {
            return Err(anyhow::anyhow!(
                "Proportion must be between 0.0 and 1.0, got {}",
                proportion
            ));
        }
        self.proportion = proportion;
        Ok(())
    }
}

/// Panel registration information
#[derive(Debug, Clone)]
pub struct PanelRegistration {
    /// Unique panel identifier
    pub panel_id: String,
    /// Human-readable panel name
    pub display_name: String,
    /// Default layout configuration
    pub default_layout: PanelLayout,
    /// Whether panel can be closed by user
    pub closeable: bool,
    /// Whether panel supports all dock positions
    pub flexible_docking: bool,
    /// Tab group identifier for multi-panel navigation (None for standalone)
    pub tab_group: Option<String>,
    /// Order within tab group (lower numbers appear first)
    pub tab_order: u32,
}

/// Tab group configuration for multi-panel navigation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TabGroup {
    /// Unique tab group identifier
    pub group_id: String,
    /// Display name for the tab group
    pub display_name: String,
    /// Position where this tab group is docked
    pub position: DockPosition,
    /// Active panel ID in this tab group
    pub active_panel_id: Option<String>,
    /// Panel IDs in this tab group (ordered by tab_order)
    pub panel_ids: Vec<String>,
}

/// Events emitted by panel manager
#[derive(Debug, Clone)]
pub enum PanelManagerEvent {
    /// Panel was registered
    PanelRegistered(String),
    /// Panel was unregistered
    PanelUnregistered(String),
    /// Panel layout changed
    LayoutChanged(String, PanelLayout),
    /// Panel visibility changed
    VisibilityChanged(String, bool),
    /// Panel position changed
    PositionChanged(String, DockPosition),
    /// All panels restored from persistence
    PanelsRestored,
    /// Tab group created
    TabGroupCreated(String),
    /// Active tab changed in a tab group
    ActiveTabChanged(String, String),
    /// Panel added to tab group
    PanelAddedToTabGroup(String, String),
    /// Panel removed from tab group
    PanelRemovedFromTabGroup(String, String),
}

/// Manages panel lifecycle, layout, and persistence
pub struct PanelManager {
    /// Registered panels with their configurations
    registrations: HashMap<String, PanelRegistration>,
    /// Current panel layouts
    layouts: HashMap<String, PanelLayout>,
    /// Tab groups for multi-panel navigation
    tab_groups: HashMap<String, TabGroup>,
    /// Panel persistence manager
    persistence: PanelPersistence,
    /// Reference to trading manager
    trading_manager: WeakEntity<TradingManager>,
    /// Active subscriptions
    _subscriptions: Vec<Subscription>,
}

impl PanelManager {
    /// Create new panel manager with proper initialization
    pub fn new(trading_manager: WeakEntity<TradingManager>, cx: &mut App) -> Entity<Self> {
        cx.new(|_cx| Self {
            registrations: HashMap::new(),
            layouts: HashMap::new(),
            tab_groups: HashMap::new(),
            persistence: PanelPersistence::new(),
            trading_manager,
            _subscriptions: Vec::new(),
        })
    }
    
    /// Register a panel with the manager
    pub fn register_panel(
        &mut self,
        registration: PanelRegistration,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        // Validate registration
        if registration.panel_id.is_empty() {
            return Err(anyhow::anyhow!("Panel ID cannot be empty"));
        }
        
        if registration.display_name.is_empty() {
            return Err(anyhow::anyhow!("Panel display name cannot be empty"));
        }
        
        // Validate default layout
        registration.default_layout.validate()?;
        
        // Check for duplicate registration with bounds checking (.rules compliance)
        if self.registrations.contains_key(&registration.panel_id) {
            return Err(anyhow::anyhow!(
                "Panel '{}' is already registered",
                registration.panel_id
            ));
        }
        
        let panel_id = registration.panel_id.clone();
        let default_layout = registration.default_layout.clone();
        let tab_group = registration.tab_group.clone();
        
        // Register panel
        self.registrations.insert(panel_id.clone(), registration);
        
        // Initialize layout from persistence or use default
        let layout = if let Some(persisted_state) = self.persistence.get_panel_state(&panel_id) {
            self.layout_from_panel_state(persisted_state, &default_layout)
        } else {
            // Create initial persistence entry for new panel
            let panel_state = PanelState {
                position: default_layout.position.clone(),
                size: default_layout.proportion * 1000.0,
                visible: default_layout.visible,
                custom_state: Default::default(),
            };
            
            // Ignore errors when setting initial state (persistence is optional)
            self.persistence.set_panel_state(panel_id.clone(), panel_state).log_err();
            
            default_layout.clone()
        };
        
        self.layouts.insert(panel_id.clone(), layout);
        
        // Add to tab group if specified
        if let Some(group_id) = tab_group {
            // Create tab group if it doesn't exist
            if !self.tab_groups.contains_key(&group_id) {
                let position = self.layouts.get(&panel_id)
                    .map(|l| l.position.clone())
                    .unwrap_or(DockPosition::Left);
                
                self.create_tab_group(
                    group_id.clone(),
                    format!("{} Group", group_id),
                    position,
                    cx,
                )?;
            }
            
            // Add panel to tab group
            self.add_panel_to_tab_group(&panel_id, &group_id, cx)?;
        }
        
        cx.emit(PanelManagerEvent::PanelRegistered(panel_id));
        cx.notify();
        
        Ok(())
    }
    
    /// Unregister a panel with proper cleanup
    pub fn unregister_panel(&mut self, panel_id: &str, cx: &mut Context<Self>) -> Result<()> {
        // Check if panel exists with bounds checking (.rules compliance)
        if !self.registrations.contains_key(panel_id) {
            return Err(anyhow::anyhow!("Panel '{}' is not registered", panel_id));
        }
        
        // Remove registration and layout
        self.registrations.remove(panel_id);
        self.layouts.remove(panel_id);
        
        cx.emit(PanelManagerEvent::PanelUnregistered(panel_id.to_string()));
        cx.notify();
        
        Ok(())
    }
    
    /// Update panel layout with validation and persistence
    pub fn update_panel_layout(
        &mut self,
        panel_id: &str,
        layout: PanelLayout,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        // Validate layout
        layout.validate()?;
        
        // Check if panel exists with bounds checking (.rules compliance)
        if !self.registrations.contains_key(panel_id) {
            return Err(anyhow::anyhow!("Panel '{}' is not registered", panel_id));
        }
        
        // Update layout
        self.layouts.insert(panel_id.to_string(), layout.clone());
        
        // Persist layout
        self.persist_panel_layout(panel_id, &layout)?;
        
        cx.emit(PanelManagerEvent::LayoutChanged(panel_id.to_string(), layout));
        cx.notify();
        
        Ok(())
    }
    
    /// Update panel position with flexible docking support
    pub fn update_panel_position(
        &mut self,
        panel_id: &str,
        position: DockPosition,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        // Get panel registration with bounds checking (.rules compliance)
        let registration = self.registrations.get(panel_id)
            .ok_or_else(|| anyhow::anyhow!("Panel '{}' is not registered", panel_id))?;
        
        // Check if panel supports flexible docking
        if !registration.flexible_docking {
            return Err(anyhow::anyhow!(
                "Panel '{}' does not support flexible docking",
                panel_id
            ));
        }
        
        // Get current layout with bounds checking (.rules compliance)
        let layout = self.layouts.get_mut(panel_id)
            .ok_or_else(|| anyhow::anyhow!("Panel '{}' has no layout", panel_id))?;
        
        layout.position = position.clone();
        
        // Persist position change
        self.persistence.update_panel_position(panel_id, position.clone())?;
        
        cx.emit(PanelManagerEvent::PositionChanged(panel_id.to_string(), position));
        cx.notify();
        
        Ok(())
    }
    
    /// Update panel visibility
    pub fn set_panel_visibility(
        &mut self,
        panel_id: &str,
        visible: bool,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        // Get layout with bounds checking (.rules compliance)
        let layout = self.layouts.get_mut(panel_id)
            .ok_or_else(|| anyhow::anyhow!("Panel '{}' is not registered", panel_id))?;
        
        layout.visible = visible;
        
        // Persist visibility change
        self.persistence.update_panel_visibility(panel_id, visible)?;
        
        cx.emit(PanelManagerEvent::VisibilityChanged(panel_id.to_string(), visible));
        cx.notify();
        
        Ok(())
    }
    
    /// Toggle panel visibility
    pub fn toggle_panel_visibility(&mut self, panel_id: &str, cx: &mut Context<Self>) -> Result<()> {
        // Get current visibility with bounds checking (.rules compliance)
        let current_visible = self.layouts.get(panel_id)
            .ok_or_else(|| anyhow::anyhow!("Panel '{}' is not registered", panel_id))?
            .visible;
        
        self.set_panel_visibility(panel_id, !current_visible, cx)
    }
    
    /// Update panel size maintaining proportional layout
    pub fn update_panel_size(
        &mut self,
        panel_id: &str,
        new_size: Pixels,
        available_space: Pixels,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        // Get layout with bounds checking (.rules compliance)
        let layout = self.layouts.get_mut(panel_id)
            .ok_or_else(|| anyhow::anyhow!("Panel '{}' is not registered", panel_id))?;
        
        // Calculate new proportion based on size
        let new_size_value = f32::from(new_size);
        let available_value = f32::from(available_space);
        let new_proportion = if available_value > 0.0 {
            (new_size_value / available_value).clamp(0.1, 0.9)
        } else {
            layout.proportion
        };
        
        layout.set_proportion(new_proportion)?;
        
        // Persist size change
        self.persistence.update_panel_size(panel_id, new_size_value)?;
        
        cx.emit(PanelManagerEvent::LayoutChanged(panel_id.to_string(), layout.clone()));
        cx.notify();
        
        Ok(())
    }
    
    /// Get panel layout with bounds checking (.rules compliance)
    pub fn get_panel_layout(&self, panel_id: &str) -> Option<&PanelLayout> {
        self.layouts.get(panel_id)
    }
    
    /// Get panel registration with bounds checking (.rules compliance)
    pub fn get_panel_registration(&self, panel_id: &str) -> Option<&PanelRegistration> {
        self.registrations.get(panel_id)
    }
    
    /// Get all registered panel IDs
    pub fn get_registered_panels(&self) -> Vec<String> {
        self.registrations.keys().cloned().collect()
    }
    
    /// Get all visible panels
    pub fn get_visible_panels(&self) -> Vec<String> {
        self.layouts
            .iter()
            .filter(|(_, layout)| layout.visible)
            .map(|(id, _)| id.clone())
            .collect()
    }
    
    /// Restore panel states from persistence
    pub fn restore_panel_states(&mut self, cx: &mut Context<Self>) -> Result<()> {
        // Load persistence data
        let config_dir = Self::get_config_directory()?;
        self.persistence = PanelPersistence::load(&config_dir)?;
        
        // Restore tab groups first
        self.restore_tab_groups()?;
        
        // Apply persisted states to registered panels
        for (panel_id, registration) in &self.registrations {
            if let Some(persisted_state) = self.persistence.get_panel_state(panel_id) {
                let layout = self.layout_from_panel_state(
                    persisted_state,
                    &registration.default_layout,
                );
                self.layouts.insert(panel_id.clone(), layout);
            }
        }
        
        cx.emit(PanelManagerEvent::PanelsRestored);
        cx.notify();
        
        Ok(())
    }
    
    /// Save all panel states to persistence
    pub fn save_panel_states(&mut self) -> Result<()> {
        let config_dir = Self::get_config_directory()?;
        self.persistence.save(&config_dir)?;
        Ok(())
    }
    
    /// Convert PanelState to PanelLayout
    fn layout_from_panel_state(&self, state: &PanelState, default: &PanelLayout) -> PanelLayout {
        PanelLayout {
            position: state.position.clone(),
            proportion: (state.size / 1000.0).clamp(0.1, 0.9),
            min_size: default.min_size,
            max_size: default.max_size,
            visible: state.visible,
        }
    }
    
    /// Persist panel layout to storage
    fn persist_panel_layout(&mut self, panel_id: &str, layout: &PanelLayout) -> Result<()> {
        let panel_state = PanelState {
            position: layout.position.clone(),
            size: layout.proportion * 1000.0,
            visible: layout.visible,
            custom_state: Default::default(),
        };
        
        self.persistence.set_panel_state(panel_id.to_string(), panel_state)?;
        
        // Save to disk
        let config_dir = Self::get_config_directory()?;
        self.persistence.save(&config_dir)?;
        
        Ok(())
    }
    
    /// Get configuration directory with proper error handling (.rules compliance)
    fn get_config_directory() -> Result<std::path::PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?
            .join("zed-lite")
            .join("stock_trading");
        
        if !config_dir.exists() {
            std::fs::create_dir_all(&config_dir)?;
        }
        
        Ok(config_dir)
    }
    
    /// Reset panel to default layout
    pub fn reset_panel_layout(&mut self, panel_id: &str, cx: &mut Context<Self>) -> Result<()> {
        // Get registration with bounds checking (.rules compliance)
        let registration = self.registrations.get(panel_id)
            .ok_or_else(|| anyhow::anyhow!("Panel '{}' is not registered", panel_id))?;
        
        let default_layout = registration.default_layout.clone();
        
        self.update_panel_layout(panel_id, default_layout, cx)
    }
    
    /// Reset all panels to default layouts
    pub fn reset_all_layouts(&mut self, cx: &mut Context<Self>) -> Result<()> {
        let panel_ids: Vec<String> = self.registrations.keys().cloned().collect();
        
        for panel_id in panel_ids {
            self.reset_panel_layout(&panel_id, cx)?;
        }
        
        Ok(())
    }
    
    /// Create a tab group for multi-panel navigation
    pub fn create_tab_group(
        &mut self,
        group_id: String,
        display_name: String,
        position: DockPosition,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        // Validate group_id
        if group_id.is_empty() {
            return Err(anyhow::anyhow!("Tab group ID cannot be empty"));
        }
        
        // Check for duplicate with bounds checking (.rules compliance)
        if self.tab_groups.contains_key(&group_id) {
            return Err(anyhow::anyhow!("Tab group '{}' already exists", group_id));
        }
        
        let tab_group = TabGroup {
            group_id: group_id.clone(),
            display_name,
            position: position.clone(),
            active_panel_id: None,
            panel_ids: Vec::new(),
        };
        
        self.tab_groups.insert(group_id.clone(), tab_group);
        
        cx.emit(PanelManagerEvent::TabGroupCreated(group_id));
        cx.notify();
        
        Ok(())
    }
    
    /// Add panel to tab group
    pub fn add_panel_to_tab_group(
        &mut self,
        panel_id: &str,
        group_id: &str,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        // Validate panel exists with bounds checking (.rules compliance)
        let _registration = self.registrations.get(panel_id)
            .ok_or_else(|| anyhow::anyhow!("Panel '{}' is not registered", panel_id))?;
        
        // Get tab group with bounds checking (.rules compliance)
        let tab_group = self.tab_groups.get_mut(group_id)
            .ok_or_else(|| anyhow::anyhow!("Tab group '{}' does not exist", group_id))?;
        
        // Check if panel already in group
        if tab_group.panel_ids.contains(&panel_id.to_string()) {
            return Err(anyhow::anyhow!(
                "Panel '{}' is already in tab group '{}'",
                panel_id,
                group_id
            ));
        }
        
        // Add panel to group
        tab_group.panel_ids.push(panel_id.to_string());
        
        // Sort panels by tab_order
        tab_group.panel_ids.sort_by_key(|id| {
            self.registrations.get(id).map(|r| r.tab_order).unwrap_or(u32::MAX)
        });
        
        // Set as active if it's the first panel
        if tab_group.active_panel_id.is_none() {
            tab_group.active_panel_id = Some(panel_id.to_string());
        }
        
        // Persist tab group state
        self.persist_tab_group_state(group_id)?;
        
        cx.emit(PanelManagerEvent::PanelAddedToTabGroup(
            panel_id.to_string(),
            group_id.to_string(),
        ));
        cx.notify();
        
        Ok(())
    }
    
    /// Remove panel from tab group
    pub fn remove_panel_from_tab_group(
        &mut self,
        panel_id: &str,
        group_id: &str,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        // Get tab group with bounds checking (.rules compliance)
        let tab_group = self.tab_groups.get_mut(group_id)
            .ok_or_else(|| anyhow::anyhow!("Tab group '{}' does not exist", group_id))?;
        
        // Remove panel from group
        let position = tab_group.panel_ids.iter().position(|id| id == panel_id)
            .ok_or_else(|| anyhow::anyhow!(
                "Panel '{}' is not in tab group '{}'",
                panel_id,
                group_id
            ))?;
        
        tab_group.panel_ids.remove(position);
        
        // Update active panel if needed
        if tab_group.active_panel_id.as_deref() == Some(panel_id) {
            tab_group.active_panel_id = tab_group.panel_ids.first().cloned();
        }
        
        // Persist tab group state
        self.persist_tab_group_state(group_id)?;
        
        cx.emit(PanelManagerEvent::PanelRemovedFromTabGroup(
            panel_id.to_string(),
            group_id.to_string(),
        ));
        cx.notify();
        
        Ok(())
    }
    
    /// Set active panel in tab group
    pub fn set_active_tab(
        &mut self,
        group_id: &str,
        panel_id: &str,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        // Get tab group with bounds checking (.rules compliance)
        let tab_group = self.tab_groups.get_mut(group_id)
            .ok_or_else(|| anyhow::anyhow!("Tab group '{}' does not exist", group_id))?;
        
        // Validate panel is in group
        if !tab_group.panel_ids.contains(&panel_id.to_string()) {
            return Err(anyhow::anyhow!(
                "Panel '{}' is not in tab group '{}'",
                panel_id,
                group_id
            ));
        }
        
        tab_group.active_panel_id = Some(panel_id.to_string());
        
        // Persist tab group state
        self.persist_tab_group_state(group_id)?;
        
        cx.emit(PanelManagerEvent::ActiveTabChanged(
            group_id.to_string(),
            panel_id.to_string(),
        ));
        cx.notify();
        
        Ok(())
    }
    
    /// Get active panel in tab group with bounds checking (.rules compliance)
    pub fn get_active_tab(&self, group_id: &str) -> Option<&String> {
        self.tab_groups.get(group_id)
            .and_then(|group| group.active_panel_id.as_ref())
    }
    
    /// Get tab group with bounds checking (.rules compliance)
    pub fn get_tab_group(&self, group_id: &str) -> Option<&TabGroup> {
        self.tab_groups.get(group_id)
    }
    
    /// Get all tab groups
    pub fn get_all_tab_groups(&self) -> Vec<&TabGroup> {
        self.tab_groups.values().collect()
    }
    
    /// Get panels in tab group with bounds checking (.rules compliance)
    pub fn get_tab_group_panels(&self, group_id: &str) -> Option<Vec<String>> {
        self.tab_groups.get(group_id)
            .map(|group| group.panel_ids.clone())
    }
    
    /// Navigate to next tab in group
    pub fn next_tab(&mut self, group_id: &str, cx: &mut Context<Self>) -> Result<()> {
        // Get tab group with bounds checking (.rules compliance)
        let tab_group = self.tab_groups.get(group_id)
            .ok_or_else(|| anyhow::anyhow!("Tab group '{}' does not exist", group_id))?;
        
        if tab_group.panel_ids.is_empty() {
            return Ok(());
        }
        
        let current_index = tab_group.active_panel_id.as_ref()
            .and_then(|id| tab_group.panel_ids.iter().position(|p| p == id))
            .unwrap_or(0);
        
        let next_index = (current_index + 1) % tab_group.panel_ids.len();
        let next_panel_id = tab_group.panel_ids.get(next_index)
            .ok_or_else(|| anyhow::anyhow!("Invalid tab index"))?
            .clone();
        
        self.set_active_tab(group_id, &next_panel_id, cx)
    }
    
    /// Navigate to previous tab in group
    pub fn previous_tab(&mut self, group_id: &str, cx: &mut Context<Self>) -> Result<()> {
        // Get tab group with bounds checking (.rules compliance)
        let tab_group = self.tab_groups.get(group_id)
            .ok_or_else(|| anyhow::anyhow!("Tab group '{}' does not exist", group_id))?;
        
        if tab_group.panel_ids.is_empty() {
            return Ok(());
        }
        
        let current_index = tab_group.active_panel_id.as_ref()
            .and_then(|id| tab_group.panel_ids.iter().position(|p| p == id))
            .unwrap_or(0);
        
        let prev_index = if current_index == 0 {
            tab_group.panel_ids.len() - 1
        } else {
            current_index - 1
        };
        
        let prev_panel_id = tab_group.panel_ids.get(prev_index)
            .ok_or_else(|| anyhow::anyhow!("Invalid tab index"))?
            .clone();
        
        self.set_active_tab(group_id, &prev_panel_id, cx)
    }
    
    /// Persist tab group state to storage
    fn persist_tab_group_state(&mut self, group_id: &str) -> Result<()> {
        // Get tab group with bounds checking (.rules compliance)
        let tab_group = self.tab_groups.get(group_id)
            .ok_or_else(|| anyhow::anyhow!("Tab group '{}' does not exist", group_id))?;
        
        // Store tab group state in panel persistence custom state
        let tab_group_key = format!("tab_group_{}", group_id);
        let tab_group_data = serde_json::to_value(tab_group)?;
        
        // Create or update a special panel state for tab groups
        let mut panel_state = self.persistence.get_panel_state(&tab_group_key)
            .cloned()
            .unwrap_or_default();
        
        panel_state.custom_state.insert("tab_group_data".to_string(), tab_group_data);
        
        self.persistence.set_panel_state(tab_group_key, panel_state)?;
        
        // Save to disk
        let config_dir = Self::get_config_directory()?;
        self.persistence.save(&config_dir)?;
        
        Ok(())
    }
    
    /// Restore tab groups from persistence
    fn restore_tab_groups(&mut self) -> Result<()> {
        // Scan persistence for tab group states
        let tab_group_keys: Vec<String> = self.persistence.panel_states.keys()
            .filter(|k| k.starts_with("tab_group_"))
            .cloned()
            .collect();
        
        for key in tab_group_keys {
            if let Some(panel_state) = self.persistence.get_panel_state(&key) {
                if let Some(tab_group_data) = panel_state.custom_state.get("tab_group_data") {
                    if let Ok(tab_group) = serde_json::from_value::<TabGroup>(tab_group_data.clone()) {
                        self.tab_groups.insert(tab_group.group_id.clone(), tab_group);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Save all panel and tab group states
    pub fn save_all_states(&mut self) -> Result<()> {
        // Save all tab group states
        let group_ids: Vec<String> = self.tab_groups.keys().cloned().collect();
        for group_id in group_ids {
            self.persist_tab_group_state(&group_id)?;
        }
        
        // Save panel states
        let config_dir = Self::get_config_directory()?;
        self.persistence.save(&config_dir)?;
        
        Ok(())
    }
}

impl EventEmitter<PanelManagerEvent> for PanelManager {}

impl Render for PanelManager {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl gpui::IntoElement {
        gpui::div()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;
    use settings::Settings;
    
    #[test]
    fn test_panel_layout_validation() {
        let mut layout = PanelLayout::default();
        assert!(layout.validate().is_ok());
        
        // Invalid proportion
        layout.proportion = 1.5;
        assert!(layout.validate().is_err());
        
        layout.proportion = 0.0;
        assert!(layout.validate().is_err());
        
        // Invalid min size
        layout.proportion = 0.3;
        layout.min_size = px(-10.0);
        assert!(layout.validate().is_err());
        
        // Invalid max size
        layout.min_size = px(200.0);
        layout.max_size = Some(px(100.0));
        assert!(layout.validate().is_err());
    }
    
    #[test]
    fn test_panel_layout_calculate_size() {
        let layout = PanelLayout {
            proportion: 0.3,
            min_size: px(200.0),
            max_size: Some(px(500.0)),
            ..Default::default()
        };
        
        // Normal case
        let size = layout.calculate_size(px(1000.0));
        assert_eq!(f32::from(size), 300.0);
        
        // Below minimum
        let size = layout.calculate_size(px(500.0));
        assert_eq!(f32::from(size), 200.0);
        
        // Above maximum
        let size = layout.calculate_size(px(2000.0));
        assert_eq!(f32::from(size), 500.0);
    }
    
    #[test]
    fn test_panel_layout_set_proportion() {
        let mut layout = PanelLayout::default();
        
        assert!(layout.set_proportion(0.5).is_ok());
        assert_eq!(layout.proportion, 0.5);
        
        assert!(layout.set_proportion(0.0).is_err());
        assert!(layout.set_proportion(1.5).is_err());
    }
    
    #[gpui::test]
    async fn test_panel_manager_registration(cx: &mut TestAppContext) {
        // Initialize SettingsStore for tests
        cx.update(|cx| {
            let settings = settings::SettingsStore::test(cx);
            cx.set_global(settings);
            StockTradingSettings::register(cx);
        });
        
        let manager = cx.update(|cx| {
            let trading_manager = TradingManager::new(
                std::sync::Arc::new(reqwest_client::ReqwestClient::new()),
                cx,
            );
            
            PanelManager::new(trading_manager.downgrade(), cx)
        });
        
        manager.update(cx, |manager, cx| {
            let registration = PanelRegistration {
                panel_id: "test_panel".to_string(),
                display_name: "Test Panel".to_string(),
                default_layout: PanelLayout::default(),
                closeable: true,
                flexible_docking: true,
                tab_group: None,
                tab_order: 0,
            };
            
            assert!(manager.register_panel(registration, cx).is_ok());
            assert!(manager.get_panel_layout("test_panel").is_some());
            
            // Duplicate registration should fail
            let duplicate = PanelRegistration {
                panel_id: "test_panel".to_string(),
                display_name: "Duplicate".to_string(),
                default_layout: PanelLayout::default(),
                closeable: true,
                flexible_docking: true,
                tab_group: None,
                tab_order: 0,
            };
            assert!(manager.register_panel(duplicate, cx).is_err());
        });
    }
    
    #[gpui::test]
    async fn test_panel_manager_layout_update(cx: &mut TestAppContext) {
        // Initialize SettingsStore for tests
        cx.update(|cx| {
            let settings = settings::SettingsStore::test(cx);
            cx.set_global(settings);
            StockTradingSettings::register(cx);
        });
        
        let manager = cx.update(|cx| {
            let trading_manager = TradingManager::new(
                std::sync::Arc::new(reqwest_client::ReqwestClient::new()),
                cx,
            );
            
            PanelManager::new(trading_manager.downgrade(), cx)
        });
        
        manager.update(cx, |manager, cx| {
            let registration = PanelRegistration {
                panel_id: "test_panel".to_string(),
                display_name: "Test Panel".to_string(),
                default_layout: PanelLayout::default(),
                closeable: true,
                flexible_docking: true,
                tab_group: None,
                tab_order: 0,
            };
            
            manager.register_panel(registration, cx).ok();
            
            let new_layout = PanelLayout {
                position: DockPosition::Right,
                proportion: 0.4,
                min_size: px(250.0),
                max_size: Some(px(600.0)),
                visible: true,
            };
            
            assert!(manager.update_panel_layout("test_panel", new_layout.clone(), cx).is_ok());
            
            let updated = manager.get_panel_layout("test_panel").unwrap();
            assert_eq!(updated.position, DockPosition::Right);
            assert_eq!(updated.proportion, 0.4);
        });
    }
    
    #[gpui::test]
    async fn test_panel_manager_visibility_toggle(cx: &mut TestAppContext) {
        // Initialize SettingsStore for tests
        cx.update(|cx| {
            let settings = settings::SettingsStore::test(cx);
            cx.set_global(settings);
            StockTradingSettings::register(cx);
        });
        
        let manager = cx.update(|cx| {
            let trading_manager = TradingManager::new(
                std::sync::Arc::new(reqwest_client::ReqwestClient::new()),
                cx,
            );
            
            PanelManager::new(trading_manager.downgrade(), cx)
        });
        
        manager.update(cx, |manager, cx| {
            let registration = PanelRegistration {
                panel_id: "test_panel".to_string(),
                display_name: "Test Panel".to_string(),
                default_layout: PanelLayout::default(),
                closeable: true,
                flexible_docking: true,
                tab_group: None,
                tab_order: 0,
            };
            
            manager.register_panel(registration, cx).ok();
            
            // Initial state is visible
            assert!(manager.get_panel_layout("test_panel").unwrap().visible);
            
            // Toggle to hidden
            let result = manager.toggle_panel_visibility("test_panel", cx);
            if let Err(ref e) = result {
                eprintln!("Toggle visibility error: {}", e);
            }
            assert!(result.is_ok());
            assert!(!manager.get_panel_layout("test_panel").unwrap().visible);
            
            // Toggle back to visible
            assert!(manager.toggle_panel_visibility("test_panel", cx).is_ok());
            assert!(manager.get_panel_layout("test_panel").unwrap().visible);
        });
    }
    
    #[gpui::test]
    async fn test_tab_group_creation(cx: &mut TestAppContext) {
        // Initialize SettingsStore for tests
        cx.update(|cx| {
            let settings = settings::SettingsStore::test(cx);
            cx.set_global(settings);
            StockTradingSettings::register(cx);
        });
        
        let manager = cx.update(|cx| {
            let trading_manager = TradingManager::new(
                std::sync::Arc::new(reqwest_client::ReqwestClient::new()),
                cx,
            );
            
            PanelManager::new(trading_manager.downgrade(), cx)
        });
        
        manager.update(cx, |manager, cx| {
            assert!(manager.create_tab_group(
                "test_group".to_string(),
                "Test Group".to_string(),
                DockPosition::Right,
                cx,
            ).is_ok());
            
            assert!(manager.get_tab_group("test_group").is_some());
            
            // Duplicate should fail
            assert!(manager.create_tab_group(
                "test_group".to_string(),
                "Duplicate".to_string(),
                DockPosition::Left,
                cx,
            ).is_err());
        });
    }
    
    #[gpui::test]
    async fn test_tab_group_panel_management(cx: &mut TestAppContext) {
        // Initialize SettingsStore for tests
        cx.update(|cx| {
            let settings = settings::SettingsStore::test(cx);
            cx.set_global(settings);
            StockTradingSettings::register(cx);
        });
        
        let manager = cx.update(|cx| {
            let trading_manager = TradingManager::new(
                std::sync::Arc::new(reqwest_client::ReqwestClient::new()),
                cx,
            );
            
            PanelManager::new(trading_manager.downgrade(), cx)
        });
        
        manager.update(cx, |manager, cx| {
            // Create tab group
            manager.create_tab_group(
                "test_group".to_string(),
                "Test Group".to_string(),
                DockPosition::Right,
                cx,
            ).ok();
            
            // Register panels
            let panel1 = PanelRegistration {
                panel_id: "panel1".to_string(),
                display_name: "Panel 1".to_string(),
                default_layout: PanelLayout::default(),
                closeable: true,
                flexible_docking: true,
                tab_group: None,
                tab_order: 0,
            };
            
            let panel2 = PanelRegistration {
                panel_id: "panel2".to_string(),
                display_name: "Panel 2".to_string(),
                default_layout: PanelLayout::default(),
                closeable: true,
                flexible_docking: true,
                tab_group: None,
                tab_order: 1,
            };
            
            manager.register_panel(panel1, cx).ok();
            manager.register_panel(panel2, cx).ok();
            
            // Add panels to tab group
            assert!(manager.add_panel_to_tab_group("panel1", "test_group", cx).is_ok());
            assert!(manager.add_panel_to_tab_group("panel2", "test_group", cx).is_ok());
            
            // Check active tab (should be first panel)
            assert_eq!(manager.get_active_tab("test_group"), Some(&"panel1".to_string()));
            
            // Set active tab
            assert!(manager.set_active_tab("test_group", "panel2", cx).is_ok());
            assert_eq!(manager.get_active_tab("test_group"), Some(&"panel2".to_string()));
            
            // Navigate tabs
            assert!(manager.next_tab("test_group", cx).is_ok());
            assert_eq!(manager.get_active_tab("test_group"), Some(&"panel1".to_string()));
            
            assert!(manager.previous_tab("test_group", cx).is_ok());
            assert_eq!(manager.get_active_tab("test_group"), Some(&"panel2".to_string()));
        });
    }
}
