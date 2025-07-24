//! Configuration loader.
//!
//! This module provides functionality to load configuration from multiple sources.

use super::{models::*, validation, ConfigError, Result, DEFAULT_CONFIG_FILES, ENV_PREFIX};
use figment::{providers::{Env, Format, Serialized, Toml, Json, Yaml}, Figment};
use std::path::{Path, PathBuf};

/// Configuration loader that handles loading from multiple sources.
#[derive(Debug, Clone)]
pub struct ConfigLoader {
    figment: Figment,
}

impl ConfigLoader {
    /// Create a new configuration loader with default values.
    pub fn new() -> Self {
        let figment = Figment::new().merge(Serialized::defaults(LocaiConfig::default()));
        Self { figment }
    }
    
    /// Load configuration from a file.
    pub fn load_file<P: AsRef<Path>>(&mut self, path: P) -> Result<&mut Self> {
        let path = path.as_ref();
        
        if !path.exists() {
            return Err(ConfigError::FileLoadError(format!(
                "Configuration file not found: {}",
                path.display()
            )));
        }
        
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("toml") => {
                let figment = std::mem::take(&mut self.figment).merge(Toml::file(path));
                self.figment = figment;
            }
            Some("yaml") | Some("yml") => {
                let figment = std::mem::take(&mut self.figment).merge(Yaml::file(path));
                self.figment = figment;
            }
            Some("json") => {
                let figment = std::mem::take(&mut self.figment).merge(Json::file(path));
                self.figment = figment;
            }
            _ => {
                return Err(ConfigError::FileLoadError(format!(
                    "Unsupported file format: {}",
                    path.display()
                )));
            }
        }
        
        Ok(self)
    }
    
    /// Attempt to load from default configuration file locations.
    pub fn load_default_files(&mut self) -> &mut Self {
        // Try to load from default file locations
        for file in DEFAULT_CONFIG_FILES {
            let path = PathBuf::from(file);
            if path.exists() {
                if let Ok(_) = self.load_file(&path) {
                    break;
                }
            }
        }
        
        // Also check XDG config directories
        if let Some(proj_dirs) = directories::ProjectDirs::from("org", "locai", "locai") {
            let config_dir = proj_dirs.config_dir();
            
            for ext in &["toml", "yaml", "yml", "json"] {
                let path = config_dir.join(format!("config.{}", ext));
                if path.exists() {
                    if let Ok(_) = self.load_file(&path) {
                        break;
                    }
                }
            }
        }
        
        self
    }
    
    /// Load configuration from environment variables.
    pub fn load_env(&mut self) -> &mut Self {
        let figment = std::mem::take(&mut self.figment)
            .merge(Env::prefixed(ENV_PREFIX).ignore(&["_"]));
        self.figment = figment;
        self
    }
    
    /// Load configuration from a custom source.
    pub fn merge<T: figment::Provider>(&mut self, provider: T) -> &mut Self {
        let figment = std::mem::take(&mut self.figment).merge(provider);
        self.figment = figment;
        self
    }
    
    /// Extract and validate the configuration.
    pub fn extract(&self) -> Result<LocaiConfig> {
        let config: LocaiConfig = self.figment.extract()
            .map_err(|e| ConfigError::ParseError(e.to_string()))?;
        
        // Validate the configuration
        validation::validate_config(&config)?;
        
        Ok(config)
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
} 