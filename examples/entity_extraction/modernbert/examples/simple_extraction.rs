use locai_modernbert_extractor::{create_modernbert_extractor_with_manager, RawEntityExtractor};
use locai::ml::ModelManagerBuilder;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("🚀 ModernBERT Entity Extraction Example");
    
    // Create a model manager
    let model_manager = Arc::new(
        ModelManagerBuilder::new()
            .build()
    );
    
    // For this example, we'll use a mock model path
    // In a real scenario, you would provide the path to your ModernBERT model
    let model_path = "/path/to/modernbert/model";
    
    println!("⚠️  Note: This example requires a trained ModernBERT model.");
    println!("📁 Expected model path: {}", model_path);
    
    // Check if model exists (in real usage)
    if !std::path::Path::new(model_path).exists() {
        println!("❌ Model not found at path: {}", model_path);
        println!("📖 To use this example:");
        println!("   1. Download or train a ModernBERT NER model");
        println!("   2. Update the model_path variable");
        println!("   3. Run the example again");
        return Ok(());
    }
    
    // Create ModernBERT extractor
    let extractor = create_modernbert_extractor_with_manager(
        model_manager.clone(),
        "modernbert-ner",
    );
    
    // Example text with entities
    let text = "Dr. John Smith works at Microsoft in Seattle. He can be reached at john.smith@microsoft.com.";
    
    println!("🔍 Extracting entities from: \"{}\"", text);
    
    // Extract entities
    let entities = extractor.extract_raw(text).await?;
    
    // Display results
    println!("✅ Found {} entities:", entities.len());
    for (i, entity) in entities.iter().enumerate() {
        println!("  {}. {} ({:?}): {:.2}", 
                 i + 1, 
                 entity.text, 
                 entity.entity_type, 
                 entity.confidence);
    }
    
    Ok(())
} 