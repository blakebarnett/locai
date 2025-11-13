//! Entity command handlers

use crate::args::*;
use crate::commands::EntityCommands;
use crate::context::LocaiCliContext;
use crate::output::*;
use colored::*;
use locai::LocaiError;
use locai::storage::filters::{EntityFilter, RelationshipFilter};
use locai::storage::models::{Entity, Relationship};
use serde_json::{Value, json};

pub async fn handle_entity_command(
    cmd: EntityCommands,
    ctx: &LocaiCliContext,
    output_format: &str,
) -> locai::Result<()> {
    match cmd {
        EntityCommands::Create(args) => {
            let properties = if let Some(props) = args.properties {
                match serde_json::from_str(&props) {
                    Ok(props) => props,
                    Err(e) => {
                        tracing::error!("Failed to parse properties JSON: {}", e);
                        return Ok(());
                    }
                }
            } else {
                Value::Null
            };

            let entity = Entity {
                id: args.id.clone(),
                entity_type: args.entity_type,
                properties,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };

            let created = ctx.memory_manager.create_entity(entity).await?;

            if output_format == "json" {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&created).unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                println!("Entity created with ID: {}", created.id);
            }
        }

        EntityCommands::Get(args) => match ctx.memory_manager.get_entity(&args.id).await? {
            Some(entity) => {
                if output_format == "json" {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&entity).unwrap_or_else(|_| "{}".to_string())
                    );
                } else {
                    print_entity(&entity);
                }
            }
            None => {
                println!("Entity with ID '{}' not found.", args.id);
            }
        },

        EntityCommands::List(args) => {
            let filter = args.entity_type.map(|entity_type| EntityFilter {
                entity_type: Some(entity_type),
                ..Default::default()
            });

            let entities = ctx
                .memory_manager
                .list_entities(filter, Some(args.limit), None)
                .await?;

            if output_format == "json" {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&entities).unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                print_entity_list(&entities);
            }
        }

        EntityCommands::Delete(args) => match ctx.memory_manager.delete_entity(&args.id).await? {
            true => println!("Entity '{}' deleted successfully.", args.id),
            false => println!("Entity '{}' not found or could not be deleted.", args.id),
        },

        EntityCommands::Count => {
            let count = ctx.memory_manager.count_entities(None).await?;

            if output_format == "json" {
                let result = json!({ "count": count });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                println!("Total entities: {}", count);
            }
        }

        EntityCommands::Update(args) => {
            let mut entity = ctx
                .memory_manager
                .get_entity(&args.id)
                .await?
                .ok_or_else(|| LocaiError::Other(format!("Entity '{}' not found", args.id)))?;

            if let Some(entity_type) = args.entity_type {
                entity.entity_type = entity_type;
            }

            if let Some(properties_str) = args.properties {
                let properties: Value = serde_json::from_str(&properties_str)
                    .map_err(|e| LocaiError::Other(format!("Invalid JSON properties: {}", e)))?;
                entity.properties = properties;
            }

            let updated = ctx.memory_manager.update_entity(entity).await?;

            if output_format == "json" {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&updated).unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                println!(
                    "{}",
                    format_success(&format!(
                        "Entity '{}' updated successfully.",
                        args.id.color(CliColors::accent())
                    ))
                );
            }
        }

        EntityCommands::Relationships(args) => {
            if let Some(command) = args.command {
                match command {
                    EntityRelationshipSubcommand::Create(create_args) => {
                        let properties = if let Some(props) = create_args.properties {
                            match serde_json::from_str::<Value>(&props) {
                                Ok(props) => props,
                                Err(e) => {
                                    output_error(
                                        &format!("Invalid JSON properties: {}", e),
                                        output_format,
                                    );
                                    return Ok(());
                                }
                            }
                        } else {
                            Value::Null
                        };

                        if ctx
                            .memory_manager
                            .get_entity(&create_args.target)
                            .await?
                            .is_none()
                        {
                            output_error(
                                &format!("Target entity '{}' not found", create_args.target),
                                output_format,
                            );
                            return Ok(());
                        }

                        let now = chrono::Utc::now();
                        let relationship = Relationship {
                            id: format!("rel:{}", uuid::Uuid::new_v4()),
                            source_id: args.id.clone(),
                            target_id: create_args.target.clone(),
                            relationship_type: create_args.relationship_type.clone(),
                            properties,
                            created_at: now,
                            updated_at: now,
                        };

                        let created = ctx
                            .memory_manager
                            .create_relationship_entity(relationship)
                            .await?;

                        if output_format == "json" {
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&created)
                                    .unwrap_or_else(|_| "{}".to_string())
                            );
                        } else {
                            println!(
                                "{}",
                                format_success(&format!(
                                    "Relationship '{}' created from entity '{}' to '{}'",
                                    create_args.relationship_type.color(CliColors::accent()),
                                    args.id.color(CliColors::accent()),
                                    create_args.target.color(CliColors::accent())
                                ))
                            );
                        }
                    }
                }
            } else {
                let outgoing = ctx
                    .memory_manager
                    .list_relationships(
                        Some(RelationshipFilter {
                            source_id: Some(args.id.clone()),
                            ..Default::default()
                        }),
                        None,
                        None,
                    )
                    .await?;

                let incoming = ctx
                    .memory_manager
                    .list_relationships(
                        Some(RelationshipFilter {
                            target_id: Some(args.id.clone()),
                            ..Default::default()
                        }),
                        None,
                        None,
                    )
                    .await?;

                if output_format == "json" {
                    let result = json!({
                        "entity_id": args.id,
                        "outgoing": outgoing,
                        "incoming": incoming,
                        "total": outgoing.len() + incoming.len()
                    });
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string())
                    );
                } else {
                    println!(
                        "{}",
                        format_info(&format!(
                            "Entity Relationships: {}",
                            args.id.color(CliColors::accent())
                        ))
                    );
                    println!();
                    if !outgoing.is_empty() {
                        println!(
                            "{}",
                            format_info(&format!("Outgoing Relationships ({}):", outgoing.len()))
                        );
                        print_relationship_list(&outgoing);
                        println!();
                    }
                    if !incoming.is_empty() {
                        println!(
                            "{}",
                            format_info(&format!("Incoming Relationships ({}):", incoming.len()))
                        );
                        print_relationship_list(&incoming);
                    }
                    if outgoing.is_empty() && incoming.is_empty() {
                        println!("{}", format_info("No relationships found."));
                    }
                }
            }
        }

        EntityCommands::Memories(args) => {
            let filter = RelationshipFilter {
                target_id: Some(args.id.clone()),
                relationship_type: Some("contains".to_string()),
                ..Default::default()
            };

            let relationships = ctx
                .memory_manager
                .list_relationships(Some(filter), Some(args.limit), None)
                .await?;

            let mut memories = Vec::new();
            for relationship in relationships {
                if let Ok(Some(memory)) =
                    ctx.memory_manager.get_memory(&relationship.source_id).await
                {
                    memories.push(memory);
                }
            }

            if output_format == "json" {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&memories).unwrap_or_else(|_| "[]".to_string())
                );
            } else {
                println!(
                    "{}",
                    format_info(&format!(
                        "Memories for Entity: {}",
                        args.id.color(CliColors::accent())
                    ))
                );
                if memories.is_empty() {
                    println!("{}", format_info("No memories found."));
                } else {
                    println!();
                    print_memory_list(&memories);
                }
            }
        }

        EntityCommands::Central(args) => {
            let entities = ctx
                .memory_manager
                .list_entities(None, Some(100), None)
                .await?;

            if entities.is_empty() {
                if output_format == "json" {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&json!({
                            "central_entities": [],
                            "total_results": 0,
                            "message": "No entities found in storage"
                        }))
                        .unwrap_or_else(|_| "{}".to_string())
                    );
                } else {
                    println!("{}", format_info("No entities found in storage."));
                }
                return Ok(());
            }

            let mut entity_centrality: Vec<(String, usize, String, String)> = Vec::new();

            for entity in entities {
                let outgoing = ctx
                    .memory_manager
                    .list_relationships(
                        Some(RelationshipFilter {
                            source_id: Some(entity.id.clone()),
                            ..Default::default()
                        }),
                        None,
                        None,
                    )
                    .await
                    .unwrap_or_default();

                let incoming = ctx
                    .memory_manager
                    .list_relationships(
                        Some(RelationshipFilter {
                            target_id: Some(entity.id.clone()),
                            ..Default::default()
                        }),
                        None,
                        None,
                    )
                    .await
                    .unwrap_or_default();

                let total_relationships = outgoing.len() + incoming.len();

                let content_preview = if let Some(name) = entity.properties.get("name") {
                    name.as_str().unwrap_or(&entity.entity_type).to_string()
                } else {
                    format!(
                        "{} ({})",
                        entity.entity_type,
                        entity.id.chars().take(8).collect::<String>()
                    )
                };

                entity_centrality.push((
                    entity.id,
                    total_relationships,
                    entity.entity_type,
                    content_preview,
                ));
            }

            entity_centrality.sort_by(|a, b| b.1.cmp(&a.1));
            entity_centrality.truncate(args.limit);

            if output_format == "json" {
                let result = json!({
                    "central_entities": entity_centrality.iter().map(|(id, score, entity_type, preview)| json!({
                        "entity_id": id,
                        "centrality_score": score,
                        "entity_type": entity_type,
                        "preview": preview
                    })).collect::<Vec<_>>()
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                println!(
                    "{}",
                    "━━━ Central Entities ━━━".color(CliColors::accent()).bold()
                );
                if entity_centrality.is_empty() {
                    println!("{}", format_info("No entities found."));
                } else {
                    println!();
                    println!(
                        "{:<8} {:<36} {:<15} {:<10} {}",
                        "Rank".color(CliColors::muted()).bold(),
                        "Entity ID".color(CliColors::muted()).bold(),
                        "Type".color(CliColors::muted()).bold(),
                        "Score".color(CliColors::muted()).bold(),
                        "Preview".color(CliColors::muted()).bold()
                    );
                    println!("{}", "─".repeat(100).color(CliColors::muted()));
                    for (i, (entity_id, score, entity_type, preview)) in
                        entity_centrality.iter().enumerate()
                    {
                        println!(
                            "{:<8} {:<36} {:<15} {:<10} {}",
                            (i + 1).to_string().color(CliColors::muted()),
                            entity_id.color(CliColors::accent()),
                            entity_type.color(CliColors::entity()),
                            score.to_string().color(CliColors::accent()),
                            preview.color(CliColors::primary())
                        );
                    }
                }
            }
        }
    }

    Ok(())
}
