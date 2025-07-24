//! Message filtering and topic matching utilities

/// Topic matcher for wildcard pattern matching
#[derive(Debug, Clone)]
pub struct TopicMatcher {
    patterns: Vec<String>,
}

impl TopicMatcher {
    /// Create a new topic matcher with the given patterns
    pub fn new(patterns: Vec<String>) -> Self {
        Self { patterns }
    }
    
    /// Check if a topic matches any of the patterns
    pub fn matches(&self, topic: &str) -> bool {
        self.patterns.iter().any(|pattern| self.match_pattern(pattern, topic))
    }
    
    /// Check if a topic matches a specific pattern
    fn match_pattern(&self, pattern: &str, topic: &str) -> bool {
        // Support wildcard patterns like "character.*", "gm.narration", etc.
        if pattern.contains('*') {
            // Simple wildcard matching
            if pattern.ends_with('*') {
                let prefix = &pattern[..pattern.len() - 1];
                return topic.starts_with(prefix);
            } else if pattern.starts_with('*') {
                let suffix = &pattern[1..];
                return topic.ends_with(suffix);
            } else {
                // For more complex patterns, we'd need proper regex
                // For now, fall back to exact match
                return pattern == topic;
            }
        }
        
        // Exact match
        pattern == topic
    }
    
    /// Add a new pattern to the matcher
    pub fn add_pattern<S: Into<String>>(&mut self, pattern: S) {
        self.patterns.push(pattern.into());
    }
    
    /// Remove a pattern from the matcher
    pub fn remove_pattern(&mut self, pattern: &str) -> bool {
        if let Some(pos) = self.patterns.iter().position(|p| p == pattern) {
            self.patterns.remove(pos);
            true
        } else {
            false
        }
    }
    
    /// Get all patterns
    pub fn patterns(&self) -> &[String] {
        &self.patterns
    }
    
    /// Check if the matcher is empty
    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }
    
    /// Clear all patterns
    pub fn clear(&mut self) {
        self.patterns.clear();
    }
}

impl Default for TopicMatcher {
    fn default() -> Self {
        Self::new(vec![])
    }
}

impl From<Vec<String>> for TopicMatcher {
    fn from(patterns: Vec<String>) -> Self {
        Self::new(patterns)
    }
}

impl From<Vec<&str>> for TopicMatcher {
    fn from(patterns: Vec<&str>) -> Self {
        Self::new(patterns.into_iter().map(|s| s.to_string()).collect())
    }
}

/// Convert a message filter to a memory filter for database queries
pub fn convert_message_filter_to_memory_filter(
    filter: &crate::messaging::types::MessageFilter,
) -> crate::Result<crate::storage::filters::MemoryFilter> {
    use crate::storage::filters::MemoryFilter;
    
    let mut memory_filter = MemoryFilter::default();
    
    // Convert topic filters to memory type filters
    if let Some(topics) = &filter.topics {
        let memory_types: Vec<String> = topics.iter()
            .filter_map(|topic| {
                // Extract topic base from namespaced topic (e.g., "app:sender.character.action" -> "character.action")
                if let Some(stripped) = topic.strip_prefix("app:") {
                    if let Some(topic_base) = stripped.split('.').nth(1) {
                        Some(format!("msg:{}", topic_base))
                    } else {
                        None
                    }
                } else {
                    Some(format!("msg:{}", topic))
                }
            })
            .collect();
        
        if !memory_types.is_empty() {
            // Use the first memory type for simplicity
            memory_filter.memory_type = Some(memory_types[0].clone());
        }
    }
    
    // Convert topic patterns to tag filters (simplified approach)
    if let Some(patterns) = &filter.topic_patterns {
        let tags: Vec<String> = patterns.iter()
            .filter_map(|pattern| {
                // Convert patterns to tag searches
                if pattern.contains('*') {
                    // Extract the base part before the wildcard
                    pattern.split('*').next().map(|s| s.to_string())
                } else {
                    Some(pattern.clone())
                }
            })
            .collect();
        
        if !tags.is_empty() {
            memory_filter.tags = Some(tags);
        }
    }
    
    // Convert sender filter to source filter
    if let Some(senders) = &filter.senders {
        if !senders.is_empty() {
            memory_filter.source = Some(senders[0].clone());
        }
    }
    
    // Convert time range
    if let Some((start, end)) = &filter.time_range {
        memory_filter.created_after = Some(*start);
        memory_filter.created_before = Some(*end);
    }
    
    // Convert tags
    if let Some(tags) = &filter.tags {
        memory_filter.tags = Some(tags.clone());
    }
    
    // Convert content query to content filter
    if let Some(query) = &filter.content_query {
        memory_filter.content = Some(query.clone());
    }
    
    Ok(memory_filter)
}

/// Extract the topic base from a full topic path
/// Example: "app:sender.character.action" -> "character.action"
pub fn extract_topic_base(full_topic: &str) -> String {
    if let Some(stripped) = full_topic.strip_prefix("app:") {
        if let Some(dot_pos) = stripped.find('.') {
            stripped[dot_pos + 1..].to_string()
        } else {
            stripped.to_string()
        }
    } else {
        full_topic.to_string()
    }
}

/// Build topic patterns for namespace-aware matching
pub fn build_namespaced_patterns(namespace: &str, patterns: &[String]) -> Vec<String> {
    patterns.iter()
        .map(|pattern| format!("{}.{}", namespace, pattern))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topic_matcher_exact() {
        let matcher = TopicMatcher::new(vec!["character.action".to_string(), "gm.narration".to_string()]);
        
        assert!(matcher.matches("character.action"));
        assert!(matcher.matches("gm.narration"));
        assert!(!matcher.matches("other.topic"));
    }
    
    #[test]
    fn test_topic_matcher_wildcard() {
        let matcher = TopicMatcher::new(vec!["character.*".to_string()]);
        
        assert!(matcher.matches("character.action"));
        assert!(matcher.matches("character.dialogue"));
        assert!(matcher.matches("character.status"));
        assert!(!matcher.matches("gm.narration"));
        assert!(!matcher.matches("world.event"));
    }
    
    #[test]
    fn test_topic_matcher_complex_patterns() {
        let matcher = TopicMatcher::new(vec![
            "*.action".to_string(),
            "character.dialogue.*".to_string(),
            "exact.match".to_string(),
        ]);
        
        // Test suffix patterns
        assert!(matcher.matches("character.action"));
        assert!(matcher.matches("npc.action"));
        
        // Test prefix patterns
        assert!(matcher.matches("character.dialogue.say"));
        assert!(matcher.matches("character.dialogue.whisper"));
        
        // Test exact match
        assert!(matcher.matches("exact.match"));
        
        assert!(!matcher.matches("character.status"));
        assert!(!matcher.matches("gm.narration"));
    }
    
    #[test]
    fn test_topic_matcher_operations() {
        let mut matcher = TopicMatcher::new(vec!["test.*".to_string()]);
        
        assert_eq!(matcher.patterns().len(), 1);
        assert!(!matcher.is_empty());
        
        matcher.add_pattern("new.pattern");
        assert_eq!(matcher.patterns().len(), 2);
        
        assert!(matcher.remove_pattern("test.*"));
        assert_eq!(matcher.patterns().len(), 1);
        
        matcher.clear();
        assert!(matcher.is_empty());
    }
    
    #[test]
    fn test_extract_topic_base() {
        assert_eq!(extract_topic_base("app:sender.character.action"), "character.action");
        assert_eq!(extract_topic_base("app:sender.gm.narration"), "gm.narration");
        assert_eq!(extract_topic_base("character.action"), "character.action");
        assert_eq!(extract_topic_base("app:sender"), "sender");
    }
    
    #[test]
    fn test_build_namespaced_patterns() {
        let patterns = vec!["character.*".to_string(), "gm.narration".to_string()];
        let namespaced = build_namespaced_patterns("app:sender", &patterns);
        
        assert_eq!(namespaced, vec![
            "app:sender.character.*",
            "app:sender.gm.narration"
        ]);
    }
    
    #[test]
    fn test_topic_matcher_from_conversions() {
        let matcher1 = TopicMatcher::from(vec!["test.*".to_string()]);
        assert!(matcher1.matches("test.topic"));
        
        let matcher2 = TopicMatcher::from(vec!["test.*", "other.*"]);
        assert!(matcher2.matches("test.topic"));
        assert!(matcher2.matches("other.topic"));
    }
} 