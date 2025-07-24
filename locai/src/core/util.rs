//! Utility functions for the core module

/// Returns a list of enabled features at compile time
/// 
/// This function helps clients determine what capabilities are available
/// in the current build of Locai.
pub fn enabled_features() -> Vec<&'static str> {
    let mut features = Vec::new();
    
    // Storage features
    #[cfg(any(feature = "surrealdb-embedded", feature = "surrealdb-remote"))]
    features.push("surrealdb");
    

    
    // API features
    #[cfg(feature = "http")]
    features.push("http");
    
    // Debugging features
    #[cfg(feature = "tokio-console")]
    features.push("tokio-console");
    
    features
}

/// Checks if a specific feature is enabled at compile time
/// 
/// # Arguments
/// * `feature` - The name of the feature to check
/// 
/// # Returns
/// * `true` if the feature is enabled, `false` otherwise
pub fn is_feature_enabled(feature: &str) -> bool {
    match feature {
        "surrealdb" => cfg!(any(feature = "surrealdb-embedded", feature = "surrealdb-remote")),
        "http" => cfg!(feature = "http"),
        "tokio-console" => cfg!(feature = "tokio-console"),
        _ => false,
    }
}

/// Determines if embedding support is available
pub fn has_embedding_support() -> bool {
    true
}

/// Check if the current build can serve HTTP requests
pub fn has_http_capability() -> bool {
    cfg!(feature = "http")
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_enabled_features() {
        let features = enabled_features();
        
        // These assertions will vary based on your test environment
        // but at least verify the function returns something
        assert!(!features.is_empty());
        
        // Verify consistency between functions
        for feature in &features {
            assert!(is_feature_enabled(feature));
        }
    }
    
    #[test]
    fn test_feature_helpers() {
        let _has_http = has_http_capability();
    }
} 