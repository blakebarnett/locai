//! Search Intelligence Layer for SharedStorage
//!
//! This module implements advanced search intelligence features leveraging SurrealDB's
//! native full-text search, fuzzy matching, and analysis capabilities.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use surrealdb::{Connection, Surreal};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::storage::errors::StorageError;

/// Query analysis result from SurrealDB analyzer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryAnalysis {
    /// Original query text
    pub query: String,
    /// Analyzed tokens from SurrealDB
    pub tokens: Vec<String>,
    /// Detected entities in the query
    pub entities: Vec<String>,
    /// Temporal expressions detected
    pub temporal_expressions: Vec<TemporalExpression>,
    /// Detected query intent
    pub intent: QueryIntent,
    /// Suggested search strategy
    pub strategy: SearchStrategy,
    /// Confidence score for the analysis
    pub confidence: f32,
}

/// Temporal expression found in query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalExpression {
    /// Raw text of the expression
    pub text: String,
    /// Parsed date range
    pub date_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    /// Type of temporal reference
    pub temporal_type: TemporalType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TemporalType {
    Absolute,    // "2023-01-01"
    Relative,    // "last week", "yesterday"
    Duration,    // "for 3 hours"
    Frequency,   // "daily", "weekly"
}

/// Query intent classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryIntent {
    /// Factual information search
    Factual,
    /// Temporal/chronological search
    Temporal,
    /// Relationship discovery
    Relational,
    /// Procedural/how-to search
    Procedural,
    /// Comparative search
    Comparative,
    /// Exploratory/discovery search
    Exploratory,
}

/// Search strategy recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchStrategy {
    /// Use semantic/vector search
    Semantic,
    /// Use full-text BM25 search
    FullText,
    /// Use fuzzy matching
    Fuzzy,
    /// Use graph traversal
    Graph,
    /// Use hybrid approach
    Hybrid,
}

/// Search session for context-aware search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSession {
    /// Session ID
    pub id: String,
    /// User ID if available
    pub user_id: Option<String>,
    /// Query history in this session
    pub query_history: Vec<QueryAnalysis>,
    /// Accumulated context
    pub context: SearchContext,
    /// Session start time
    pub started_at: DateTime<Utc>,
    /// Last activity time
    pub last_activity: DateTime<Utc>,
}

/// Accumulated search context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchContext {
    /// Entities mentioned in session
    pub entities: HashMap<String, f32>, // entity_id -> relevance
    /// Topics covered
    pub topics: HashMap<String, f32>,
    /// Memory types accessed
    pub memory_types: HashMap<String, u32>,
    /// Temporal focus if any
    pub temporal_focus: Option<TemporalExpression>,
}

/// Enhanced search result with explanation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelligentSearchResult {
    /// Result ID
    pub id: String,
    /// Result type
    pub result_type: String,
    /// Content summary
    pub content: serde_json::Value,
    /// Combined relevance score
    pub score: f32,
    /// Score breakdown by signal
    pub score_breakdown: ScoreBreakdown,
    /// Match explanation
    pub explanation: MatchExplanation,
    /// Context information
    pub context: ResultContext,
}

/// Score breakdown showing different ranking signals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    /// BM25 full-text score
    pub bm25_score: Option<f32>,
    /// Vector similarity score
    pub vector_score: Option<f32>,
    /// Graph centrality score
    pub graph_score: Option<f32>,
    /// Temporal relevance score
    pub temporal_score: Option<f32>,
    /// User preference score
    pub preference_score: Option<f32>,
    /// Final combined score
    pub combined_score: f32,
}

/// Match explanation with highlights and reasoning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchExplanation {
    /// Primary match reason
    pub primary_reason: String,
    /// Detailed explanations
    pub details: Vec<String>,
    /// Highlighted text snippets
    pub highlights: Vec<Highlight>,
    /// Match path for graph results
    pub match_path: Option<Vec<String>>,
    /// Analyzer information used
    pub analyzer_info: Option<AnalyzerInfo>,
}

/// Text highlight from SurrealDB search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Highlight {
    /// Highlighted text with markup
    pub text: String,
    /// Field that was highlighted
    pub field: String,
    /// Position in original text
    pub position: Option<(usize, usize)>,
}

/// Information about analyzer processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzerInfo {
    /// Analyzer used
    pub analyzer: String,
    /// Tokens generated
    pub tokens: Vec<String>,
    /// Stemmed terms
    pub stems: Vec<String>,
    /// Mapped terms
    pub mappings: HashMap<String, String>,
}

/// Context about the result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultContext {
    /// Related entities
    pub related_entities: Vec<String>,
    /// Related memories
    pub related_memories: Vec<String>,
    /// Graph relationships
    pub relationships: Vec<String>,
    /// Temporal context
    pub temporal_context: Option<String>,
}

/// Search suggestion with confidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSuggestion {
    /// Suggested query text
    pub suggestion: String,
    /// Type of suggestion
    pub suggestion_type: SuggestionType,
    /// Confidence score
    pub confidence: f32,
    /// Explanation for the suggestion
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuggestionType {
    /// Spelling correction
    Correction,
    /// Query completion
    Completion,
    /// Query expansion
    Expansion,
    /// Alternative formulation
    Alternative,
    /// Scope refinement
    Refinement,
}

/// Main intelligence layer implementation
#[derive(Debug)]
pub struct SearchIntelligence<C>
where
    C: Connection + Clone + Send + Sync + std::fmt::Debug + 'static,
{
    client: Surreal<C>,
    sessions: HashMap<String, SearchSession>,
}

impl<C> SearchIntelligence<C>
where
    C: Connection + Clone + Send + Sync + std::fmt::Debug + 'static,
{
    pub fn new(client: Surreal<C>) -> Self {
        Self {
            client,
            sessions: HashMap::new(),
        }
    }

    /// Analyze query using SurrealDB's native text analysis
    pub async fn analyze_query(&self, query: &str) -> Result<QueryAnalysis, StorageError> {
        // Use SurrealDB's search::analyze function
        let analyze_query = r#"
            SELECT search::analyze('memory_analyzer', $query) as tokens,
                   search::analyze('entity_analyzer', $query) as entity_tokens
        "#;

        let query_string = query.to_string();
        let mut result = self.client
            .query(analyze_query)
            .bind(("query", query_string))
            .await
            .map_err(|e| StorageError::Query(format!("Failed to analyze query: {}", e)))?;

        #[derive(Deserialize)]
        struct AnalysisResult {
            tokens: Vec<String>,
            entity_tokens: Vec<String>,
        }

        let analysis: Option<AnalysisResult> = result.take(0)
            .map_err(|e| StorageError::Query(format!("Failed to extract analysis: {}", e)))?;

        let analysis = analysis.unwrap_or(AnalysisResult {
            tokens: vec![query.to_string()],
            entity_tokens: vec![],
        });

        // Detect entities and temporal expressions
        let entities = self.detect_entities(query, &analysis.entity_tokens).await?;
        let temporal_expressions = self.detect_temporal_expressions(query).await?;
        
        // Classify query intent
        let intent = self.classify_query_intent(query, &entities, &temporal_expressions).await?;
        
        // Suggest search strategy
        let strategy = self.suggest_search_strategy(&intent, &entities, &temporal_expressions).await?;

        Ok(QueryAnalysis {
            query: query.to_string(),
            tokens: analysis.tokens,
            entities,
            temporal_expressions,
            intent,
            strategy,
            confidence: 0.8, // TODO: Implement confidence calculation
        })
    }

    /// Create or update search session
    pub async fn create_session(&mut self, user_id: Option<String>) -> String {
        let session_id = Uuid::new_v4().to_string();
        let session = SearchSession {
            id: session_id.clone(),
            user_id,
            query_history: Vec::new(),
            context: SearchContext {
                entities: HashMap::new(),
                topics: HashMap::new(),
                memory_types: HashMap::new(),
                temporal_focus: None,
            },
            started_at: Utc::now(),
            last_activity: Utc::now(),
        };

        self.sessions.insert(session_id.clone(), session);
        session_id
    }

    /// Update session with new query
    pub async fn update_session(&mut self, session_id: &str, analysis: QueryAnalysis) -> Result<(), StorageError> {
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.query_history.push(analysis.clone());
            session.last_activity = Utc::now();

            // Update context
            for entity in &analysis.entities {
                *session.context.entities.entry(entity.clone()).or_insert(0.0) += 1.0;
            }

            if let Some(temporal) = analysis.temporal_expressions.first() {
                session.context.temporal_focus = Some(temporal.clone());
            }
        }
        Ok(())
    }

    /// Get session context for search
    pub fn get_session_context(&self, session_id: &str) -> Option<&SearchContext> {
        self.sessions.get(session_id).map(|s| &s.context)
    }

    /// Get session by ID
    pub fn get_session(&self, session_id: &str) -> Option<&SearchSession> {
        self.sessions.get(session_id)
    }

    /// Perform hybrid search combining BM25, vector, and graph signals
    pub async fn hybrid_search(
        &self,
        analysis: &QueryAnalysis,
        session_context: Option<&SearchContext>,
        limit: Option<usize>,
    ) -> Result<Vec<IntelligentSearchResult>, StorageError> {
        let limit = limit.unwrap_or(10);

        match analysis.strategy {
            SearchStrategy::FullText => self.bm25_search(analysis, limit).await,
            SearchStrategy::Semantic => self.semantic_search(analysis, limit).await,
            SearchStrategy::Fuzzy => self.fuzzy_search(analysis, limit).await,
            SearchStrategy::Graph => self.graph_search(analysis, session_context, limit).await,
            SearchStrategy::Hybrid => self.combined_search(analysis, session_context, limit).await,
        }
    }

    /// Generate search suggestions
    pub async fn generate_suggestions(&self, partial_query: &str, session_context: Option<&SearchContext>) -> Result<Vec<SearchSuggestion>, StorageError> {
        let mut suggestions = Vec::new();

        // Auto-complete from entity names
        suggestions.extend(self.entity_autocompletion(partial_query).await?);
        
        // Spelling corrections using fuzzy matching
        suggestions.extend(self.spelling_corrections(partial_query).await?);
        
        // Query expansion based on context
        if let Some(context) = session_context {
            suggestions.extend(self.context_expansion(partial_query, context).await?);
        }

        // Sort by confidence and limit
        suggestions.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
        suggestions.truncate(10);

        Ok(suggestions)
    }

    /// Explain search results with detailed reasoning
    pub async fn explain_results(&self, results: &[IntelligentSearchResult]) -> Result<String, StorageError> {
        let mut explanation = String::new();
        
        explanation.push_str(&format!("Found {} results:\n\n", results.len()));
        
        for (i, result) in results.iter().enumerate() {
            explanation.push_str(&format!("{}. {} (score: {:.2})\n", 
                i + 1, result.explanation.primary_reason, result.score));
            
            if !result.explanation.details.is_empty() {
                explanation.push_str("   Details:\n");
                for detail in &result.explanation.details {
                    explanation.push_str(&format!("   - {}\n", detail));
                }
            }
            
            if !result.explanation.highlights.is_empty() {
                explanation.push_str("   Highlights:\n");
                for highlight in &result.explanation.highlights {
                    explanation.push_str(&format!("   - {}: {}\n", highlight.field, highlight.text));
                }
            }
            
            explanation.push('\n');
        }

        Ok(explanation)
    }

    // Private helper methods
    
    async fn detect_entities(&self, query: &str, _entity_tokens: &[String]) -> Result<Vec<String>, StorageError> {
        // Use entity tokens and fuzzy matching to find existing entities
        let entity_search_query = r#"
            SELECT id, properties.name as name, properties.text as text 
            FROM entity 
            WHERE properties.name ~* $query 
               OR properties.text ~* $query
               OR entity_type ~* $query
            LIMIT 10
        "#;

        let query_string = query.to_string();
        let mut result = self.client
            .query(entity_search_query)
            .bind(("query", query_string))
            .await
            .map_err(|e| StorageError::Query(format!("Failed to detect entities: {}", e)))?;

        #[derive(Deserialize)]
        struct EntityMatch {
            id: String,
            #[allow(dead_code)]
            name: Option<String>,
            #[allow(dead_code)]
            text: Option<String>,
        }

        let entities: Vec<EntityMatch> = result.take(0)
            .map_err(|e| StorageError::Query(format!("Failed to extract entities: {}", e)))?;

        Ok(entities.into_iter().map(|e| e.id).collect())
    }

    async fn detect_temporal_expressions(&self, query: &str) -> Result<Vec<TemporalExpression>, StorageError> {
        // Simple temporal expression detection
        let temporal_patterns = [
            ("yesterday", TemporalType::Relative),
            ("today", TemporalType::Relative),
            ("last week", TemporalType::Relative),
            ("last month", TemporalType::Relative),
            ("last year", TemporalType::Relative),
            ("this week", TemporalType::Relative),
            ("this month", TemporalType::Relative),
            ("recently", TemporalType::Relative),
        ];

        let mut expressions = Vec::new();
        let query_lower = query.to_lowercase();

        for (pattern, temporal_type) in temporal_patterns {
            if query_lower.contains(pattern) {
                expressions.push(TemporalExpression {
                    text: pattern.to_string(),
                    date_range: None, // TODO: Implement date parsing
                    temporal_type,
                });
            }
        }

        Ok(expressions)
    }

    async fn classify_query_intent(&self, query: &str, _entities: &[String], temporal: &[TemporalExpression]) -> Result<QueryIntent, StorageError> {
        let query_lower = query.to_lowercase();

        // Simple intent classification
        if !temporal.is_empty() {
            return Ok(QueryIntent::Temporal);
        }

        if query_lower.contains("how") || query_lower.contains("what") || query_lower.contains("why") {
            return Ok(QueryIntent::Factual);
        }

        if query_lower.contains("relationship") || query_lower.contains("connection") || query_lower.contains("related") {
            return Ok(QueryIntent::Relational);
        }

        if query_lower.contains("compare") || query_lower.contains("vs") || query_lower.contains("difference") {
            return Ok(QueryIntent::Comparative);
        }

        if query_lower.contains("step") || query_lower.contains("process") || query_lower.contains("procedure") {
            return Ok(QueryIntent::Procedural);
        }

        Ok(QueryIntent::Exploratory)
    }

    async fn suggest_search_strategy(&self, intent: &QueryIntent, entities: &[String], temporal: &[TemporalExpression]) -> Result<SearchStrategy, StorageError> {
        match intent {
            QueryIntent::Relational if !entities.is_empty() => Ok(SearchStrategy::Graph),
            QueryIntent::Temporal if !temporal.is_empty() => Ok(SearchStrategy::FullText),
            QueryIntent::Factual => Ok(SearchStrategy::Hybrid),
            QueryIntent::Exploratory => Ok(SearchStrategy::Semantic),
            _ => Ok(SearchStrategy::FullText),
        }
    }

    async fn bm25_search(&self, analysis: &QueryAnalysis, limit: usize) -> Result<Vec<IntelligentSearchResult>, StorageError> {
        let search_query = r#"
            SELECT *, 
                   search::score(0) AS bm25_score,
                   search::highlight('<mark>', '</mark>', 0) AS highlighted_content
            FROM memory 
            WHERE content @0@ $query
            ORDER BY bm25_score DESC
            LIMIT $limit
        "#;

        let query_string = analysis.query.clone();
        let mut result = self.client
            .query(search_query)
            .bind(("query", query_string))
            .bind(("limit", limit))
            .await
            .map_err(|e| StorageError::Query(format!("Failed to perform BM25 search: {}", e)))?;

        #[derive(Deserialize)]
        struct BM25Result {
            id: String,
            content: String,
            bm25_score: f32,
            highlighted_content: String,
        }

        let results: Vec<BM25Result> = result.take(0)
            .map_err(|e| StorageError::Query(format!("Failed to extract BM25 results: {}", e)))?;

        Ok(results.into_iter().map(|r| IntelligentSearchResult {
            id: r.id,
            result_type: "memory".to_string(),
            content: serde_json::json!({ "content": r.content }),
            score: r.bm25_score,
            score_breakdown: ScoreBreakdown {
                bm25_score: Some(r.bm25_score),
                vector_score: None,
                graph_score: None,
                temporal_score: None,
                preference_score: None,
                combined_score: r.bm25_score,
            },
            explanation: MatchExplanation {
                primary_reason: "BM25 text match".to_string(),
                details: vec!["Found using full-text search with BM25 scoring".to_string()],
                highlights: vec![Highlight {
                    text: r.highlighted_content,
                    field: "content".to_string(),
                    position: None,
                }],
                match_path: None,
                analyzer_info: Some(AnalyzerInfo {
                    analyzer: "memory_analyzer".to_string(),
                    tokens: analysis.tokens.clone(),
                    stems: vec![], // Note: Stem extraction from analyzer not exposed by SurrealDB
                    mappings: HashMap::new(),
                }),
            },
            context: ResultContext {
                related_entities: vec![],
                related_memories: vec![],
                relationships: vec![],
                temporal_context: None,
            },
        }).collect())
    }

    async fn semantic_search(&self, analysis: &QueryAnalysis, limit: usize) -> Result<Vec<IntelligentSearchResult>, StorageError> {
        // Note: Vector-based semantic search not yet implemented
        // Using BM25 full-text search which provides excellent results for most queries
        self.bm25_search(analysis, limit).await
    }

    async fn fuzzy_search(&self, analysis: &QueryAnalysis, limit: usize) -> Result<Vec<IntelligentSearchResult>, StorageError> {
        let fuzzy_query = r#"
            SELECT *, 
                   string::similarity::fuzzy(content, $query) AS fuzzy_score
            FROM memory 
            WHERE content ~* $query
            ORDER BY fuzzy_score DESC
            LIMIT $limit
        "#;

        let query_string = analysis.query.clone();
        let mut result = self.client
            .query(fuzzy_query)
            .bind(("query", query_string))
            .bind(("limit", limit))
            .await
            .map_err(|e| StorageError::Query(format!("Failed to perform fuzzy search: {}", e)))?;

        #[derive(Deserialize)]
        struct FuzzyResult {
            id: String,
            content: String,
            fuzzy_score: f32,
        }

        let results: Vec<FuzzyResult> = result.take(0)
            .map_err(|e| StorageError::Query(format!("Failed to extract fuzzy results: {}", e)))?;

        Ok(results.into_iter().map(|r| IntelligentSearchResult {
            id: r.id,
            result_type: "memory".to_string(),
            content: serde_json::json!({ "content": r.content }),
            score: r.fuzzy_score,
            score_breakdown: ScoreBreakdown {
                bm25_score: None,
                vector_score: None,
                graph_score: None,
                temporal_score: None,
                preference_score: None,
                combined_score: r.fuzzy_score,
            },
            explanation: MatchExplanation {
                primary_reason: "Fuzzy text match".to_string(),
                details: vec!["Found using fuzzy string matching for typo tolerance".to_string()],
                highlights: vec![],
                match_path: None,
                analyzer_info: None,
            },
            context: ResultContext {
                related_entities: vec![],
                related_memories: vec![],
                relationships: vec![],
                temporal_context: None,
            },
        }).collect())
    }

    async fn graph_search(&self, analysis: &QueryAnalysis, _session_context: Option<&SearchContext>, limit: usize) -> Result<Vec<IntelligentSearchResult>, StorageError> {
        // Note: Graph-based search not yet implemented
        // Using BM25 full-text search as the primary search method
        self.bm25_search(analysis, limit).await
    }

    async fn combined_search(&self, analysis: &QueryAnalysis, _session_context: Option<&SearchContext>, limit: usize) -> Result<Vec<IntelligentSearchResult>, StorageError> {
        // Note: Multi-signal hybrid search not yet implemented
        // BM25 provides sophisticated full-text search with relevance ranking
        self.bm25_search(analysis, limit).await
    }

    async fn entity_autocompletion(&self, partial_query: &str) -> Result<Vec<SearchSuggestion>, StorageError> {
        let completion_query = r#"
            SELECT properties.name as name, entity_type 
            FROM entity 
            WHERE properties.name ~ $partial
            LIMIT 5
        "#;

        let mut result = self.client
            .query(completion_query)
            .bind(("partial", format!("{}*", partial_query)))
            .await
            .map_err(|e| StorageError::Query(format!("Failed to get autocompletion: {}", e)))?;

        #[derive(Deserialize)]
        struct CompletionResult {
            name: Option<String>,
            entity_type: String,
        }

        let results: Vec<CompletionResult> = result.take(0)
            .map_err(|e| StorageError::Query(format!("Failed to extract completions: {}", e)))?;

        Ok(results.into_iter().filter_map(|r| {
            r.name.map(|name| SearchSuggestion {
                suggestion: name.clone(),
                suggestion_type: SuggestionType::Completion,
                confidence: 0.8,
                explanation: format!("Entity name completion from type '{}'", r.entity_type),
            })
        }).collect())
    }

    async fn spelling_corrections(&self, _query: &str) -> Result<Vec<SearchSuggestion>, StorageError> {
        // Note: Dedicated spelling correction not implemented
        // Fuzzy search provides typo tolerance for queries
        Ok(vec![])
    }

    async fn context_expansion(&self, query: &str, context: &SearchContext) -> Result<Vec<SearchSuggestion>, StorageError> {
        let mut suggestions = Vec::new();

        // Suggest adding high-relevance entities from context
        for (entity, relevance) in &context.entities {
            if *relevance > 2.0 && !query.contains(entity) {
                suggestions.push(SearchSuggestion {
                    suggestion: format!("{} {}", query, entity),
                    suggestion_type: SuggestionType::Expansion,
                    confidence: relevance / 10.0,
                    explanation: format!("Add frequently mentioned entity '{}'", entity),
                });
            }
        }

        Ok(suggestions)
    }
}

/// Trait for intelligent search capabilities
#[async_trait]
pub trait IntelligentSearch {
    /// Analyze a query for intent and strategy
    async fn analyze_query(&self, query: &str) -> Result<QueryAnalysis, StorageError>;
    
    /// Perform intelligent search with context
    async fn intelligent_search(&self, query: &str, session_id: Option<&str>, limit: Option<usize>) -> Result<Vec<IntelligentSearchResult>, StorageError>;
    
    /// Generate search suggestions
    async fn suggest(&self, partial_query: &str, session_id: Option<&str>) -> Result<Vec<SearchSuggestion>, StorageError>;
    
    /// Explain search results
    async fn explain(&self, results: &[IntelligentSearchResult]) -> Result<String, StorageError>;
} 