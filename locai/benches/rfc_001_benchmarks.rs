//! Performance benchmarks for RFC 001 features
//!
//! Run with: cargo bench --bench rfc_001_benchmarks

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use locai::config::LifecycleTrackingConfig;
use locai::hooks::{HookRegistry, HookResult, MemoryHook};
use locai::models::{Memory, MemoryPriority, MemoryType};
use locai::relationships::registry::{RelationshipTypeDef, RelationshipTypeRegistry};
use locai::search::scoring::{DecayFunction, ScoringConfig};
use locai::search::calculator::ScoreCalculator;
use locai::storage::lifecycle::{LifecycleUpdate, LifecycleUpdateQueue};
use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;

/// Create a test memory for benchmarking
fn create_bench_memory(id: &str) -> Memory {
    Memory {
        id: id.to_string(),
        content: "Benchmark memory content for performance testing".to_string(),
        memory_type: MemoryType::Fact,
        created_at: Utc::now(),
        last_accessed: None,
        access_count: 0,
        priority: MemoryPriority::Normal,
        tags: vec!["benchmark".to_string(), "test".to_string()],
        source: "bench".to_string(),
        expires_at: None,
        properties: serde_json::json!({"test": "value"}),
        related_memories: vec![],
        embedding: None,
    }
}

/// No-op hook for benchmarking
#[derive(Debug)]
struct NoOpHook;

#[async_trait]
impl MemoryHook for NoOpHook {
    async fn on_memory_created(&self, _memory: &Memory) -> HookResult {
        HookResult::Continue
    }
    
    async fn on_memory_accessed(&self, _memory: &Memory) -> HookResult {
        HookResult::Continue
    }
    
    fn name(&self) -> &str {
        "noop_hook"
    }
}

/// Async hook that simulates actual work
#[derive(Debug)]
struct WorkHook {
    delay_us: u64,
}

#[async_trait]
impl MemoryHook for WorkHook {
    async fn on_memory_created(&self, _memory: &Memory) -> HookResult {
        tokio::time::sleep(std::time::Duration::from_micros(self.delay_us)).await;
        HookResult::Continue
    }
    
    fn name(&self) -> &str {
        "work_hook"
    }
}

// =============================================================================
// Benchmark 1: Lifecycle Tracking Overhead
// =============================================================================

fn bench_lifecycle_tracking_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("lifecycle_tracking");
    
    // Baseline: no tracking
    group.bench_function("baseline_no_tracking", |b| {
        b.iter(|| {
            let memory = create_bench_memory("bench_1");
            black_box(memory);
        });
    });
    
    // With record_access (in-memory only)
    group.bench_function("with_record_access", |b| {
        b.iter(|| {
            let mut memory = create_bench_memory("bench_1");
            memory.record_access();
            black_box(memory);
        });
    });
    
    // Queuing lifecycle updates
    group.bench_function("queue_update", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let queue = LifecycleUpdateQueue::new(1000);
        
        b.to_async(&rt).iter(|| async {
            let update = LifecycleUpdate::new("mem_123".to_string());
            queue.queue_update(update).await.ok();
        });
    });
    
    // Queue merging
    group.bench_function("queue_merge_updates", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let queue = LifecycleUpdateQueue::new(1000);
        
        b.to_async(&rt).iter(|| async {
            for _ in 0..10 {
                let update = LifecycleUpdate::new("mem_123".to_string());
                queue.queue_update(update).await.ok();
            }
            queue.drain().await;
        });
    });
    
    group.finish();
}

// =============================================================================
// Benchmark 2: Hook Execution Overhead
// =============================================================================

fn bench_hook_execution(c: &mut Criterion) {
    let mut group = c.benchmark_group("hook_execution");
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    // No hooks
    group.bench_function("0_hooks", |b| {
        let registry = HookRegistry::new();
        let memory = create_bench_memory("bench_1");
        
        b.to_async(&rt).iter(|| async {
            registry.execute_on_created(&memory).await.ok();
        });
    });
    
    // 1 no-op hook
    group.bench_function("1_noop_hook", |b| {
        let registry = Arc::new(HookRegistry::new());
        rt.block_on(async {
            registry.register(Arc::new(NoOpHook)).await;
        });
        let memory = create_bench_memory("bench_1");
        
        b.to_async(&rt).iter(|| async {
            registry.execute_on_created(&memory).await.ok();
        });
    });
    
    // 5 no-op hooks
    group.bench_function("5_noop_hooks", |b| {
        let registry = Arc::new(HookRegistry::new());
        rt.block_on(async {
            for _ in 0..5 {
                registry.register(Arc::new(NoOpHook)).await;
            }
        });
        let memory = create_bench_memory("bench_1");
        
        b.to_async(&rt).iter(|| async {
            registry.execute_on_created(&memory).await.ok();
        });
    });
    
    // 10 no-op hooks
    group.bench_function("10_noop_hooks", |b| {
        let registry = Arc::new(HookRegistry::new());
        rt.block_on(async {
            for _ in 0..10 {
                registry.register(Arc::new(NoOpHook)).await;
            }
        });
        let memory = create_bench_memory("bench_1");
        
        b.to_async(&rt).iter(|| async {
            registry.execute_on_created(&memory).await.ok();
        });
    });
    
    // 1 hook with simulated work (100µs)
    group.bench_function("1_work_hook_100us", |b| {
        let registry = Arc::new(HookRegistry::new());
        rt.block_on(async {
            registry.register(Arc::new(WorkHook { delay_us: 100 })).await;
        });
        let memory = create_bench_memory("bench_1");
        
        b.to_async(&rt).iter(|| async {
            registry.execute_on_created(&memory).await.ok();
        });
    });
    
    group.finish();
}

// =============================================================================
// Benchmark 3: Relationship Registry Operations
// =============================================================================

fn bench_relationship_registry(c: &mut Criterion) {
    let mut group = c.benchmark_group("relationship_registry");
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    // Register a type
    group.bench_function("register_type", |b| {
        b.to_async(&rt).iter(|| async {
            let registry = RelationshipTypeRegistry::new();
            let type_def = RelationshipTypeDef::new("test_type".to_string()).unwrap();
            registry.register(type_def).await.ok();
        });
    });
    
    // Get a type (cache hit)
    group.bench_function("get_type", |b| {
        let registry = RelationshipTypeRegistry::new();
        rt.block_on(async {
            let type_def = RelationshipTypeDef::new("test_type".to_string()).unwrap();
            registry.register(type_def).await.ok();
        });
        
        b.to_async(&rt).iter(|| async {
            registry.get("test_type").await;
        });
    });
    
    // List types with varying counts
    for count in [10, 50, 100, 500].iter() {
        group.bench_with_input(BenchmarkId::new("list_types", count), count, |b, &count| {
            let registry = RelationshipTypeRegistry::new();
            rt.block_on(async {
                for i in 0..count {
                    let type_def = RelationshipTypeDef::new(format!("type_{}", i)).unwrap();
                    registry.register(type_def).await.ok();
                }
            });
            
            b.to_async(&rt).iter(|| async {
                let _types = registry.list().await;
            });
        });
    }
    
    // Seed common types
    group.bench_function("seed_common_types", |b| {
        b.to_async(&rt).iter(|| async {
            let registry = RelationshipTypeRegistry::new();
            registry.seed_common_types().await.ok();
        });
    });
    
    group.finish();
}

// =============================================================================
// Benchmark 4: Enhanced Search Scoring
// =============================================================================

fn bench_enhanced_search_scoring(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_scoring");
    
    // Baseline: no scoring
    group.bench_function("baseline_no_scoring", |b| {
        let memory = create_bench_memory("bench_1");
        let base_score = 10.0;
        
        b.iter(|| {
            black_box(base_score);
            black_box(&memory);
        });
    });
    
    // Basic scoring (BM25 only)
    group.bench_function("bm25_only", |b| {
        let config = ScoringConfig {
            bm25_weight: 1.0,
            vector_weight: 0.0,
            recency_boost: 0.0,
            access_boost: 0.0,
            priority_boost: 0.0,
            decay_function: DecayFunction::None,
            decay_rate: 0.1,
        };
        let calc = ScoreCalculator::new(config);
        let memory = create_bench_memory("bench_1");
        
        b.iter(|| {
            let score = calc.calculate_final_score(10.0, None, &memory);
            black_box(score);
        });
    });
    
    // Full scoring with all boosts
    group.bench_function("full_scoring", |b| {
        let config = ScoringConfig::default();
        let calc = ScoreCalculator::new(config);
        let memory = create_bench_memory("bench_1");
        
        b.iter(|| {
            let score = calc.calculate_final_score(10.0, Some(8.0), &memory);
            black_box(score);
        });
    });
    
    // Scoring with each decay function
    for (name, decay_fn) in [
        ("none", DecayFunction::None),
        ("linear", DecayFunction::Linear),
        ("exponential", DecayFunction::Exponential),
        ("logarithmic", DecayFunction::Logarithmic),
    ].iter() {
        group.bench_with_input(BenchmarkId::new("decay_function", name), decay_fn, |b, &decay_fn| {
            let config = ScoringConfig {
                bm25_weight: 0.5,
                vector_weight: 0.5,
                recency_boost: 1.0,
                access_boost: 0.5,
                priority_boost: 0.3,
                decay_function: decay_fn,
                decay_rate: 0.1,
            };
            let calc = ScoreCalculator::new(config);
            let memory = create_bench_memory("bench_1");
            
            b.iter(|| {
                let score = calc.calculate_final_score(10.0, Some(8.0), &memory);
                black_box(score);
            });
        });
    }
    
    // Batch scoring (simulate scoring 100 results)
    group.bench_function("batch_100_memories", |b| {
        let config = ScoringConfig::default();
        let calc = ScoreCalculator::new(config);
        let memories: Vec<Memory> = (0..100)
            .map(|i| create_bench_memory(&format!("mem_{}", i)))
            .collect();
        
        b.iter(|| {
            let scores: Vec<f32> = memories.iter()
                .map(|m| calc.calculate_final_score(10.0, Some(8.0), m))
                .collect();
            black_box(scores);
        });
    });
    
    group.finish();
}

// =============================================================================
// Benchmark 5: Configuration Validation
// =============================================================================

fn bench_configuration_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("config_validation");
    
    // Lifecycle config validation
    group.bench_function("lifecycle_config_validate", |b| {
        let config = LifecycleTrackingConfig::default();
        
        b.iter(|| {
            black_box(config.validate());
        });
    });
    
    // Scoring config validation
    group.bench_function("scoring_config_validate", |b| {
        let config = ScoringConfig::default();
        
        b.iter(|| {
            black_box(config.validate());
        });
    });
    
    // Scoring config normalization
    group.bench_function("scoring_config_normalize", |b| {
        b.iter(|| {
            let mut config = ScoringConfig::default();
            config.normalize_weights();
            black_box(config);
        });
    });
    
    group.finish();
}

// =============================================================================
// Benchmark 6: Memory Access Patterns
// =============================================================================

fn bench_memory_access_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_access_patterns");
    
    // Sequential access (simulates typical usage)
    group.bench_function("sequential_access", |b| {
        let mut memory = create_bench_memory("bench_1");
        
        b.iter(|| {
            memory.record_access();
            black_box(&memory);
        });
    });
    
    // Concurrent access simulation
    group.bench_function("concurrent_access_sim", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        
        b.to_async(&rt).iter(|| async {
            let mut tasks = Vec::new();
            
            for _ in 0..10 {
                let task = tokio::spawn(async {
                    let mut memory = create_bench_memory("bench_1");
                    memory.record_access();
                    black_box(memory);
                });
                tasks.push(task);
            }
            
            for task in tasks {
                task.await.ok();
            }
        });
    });
    
    group.finish();
}

// =============================================================================
// Benchmark Group Configuration
// =============================================================================

criterion_group!(
    benches,
    bench_lifecycle_tracking_overhead,
    bench_hook_execution,
    bench_relationship_registry,
    bench_enhanced_search_scoring,
    bench_configuration_validation,
    bench_memory_access_patterns,
);

criterion_main!(benches);






