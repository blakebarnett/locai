//! Relationship command handlers

use crate::commands::RelationshipCommands;
use crate::context::LocaiCliContext;
use crate::output::*;
use colored::Colorize;
use locai::LocaiError;
use locai::storage::filters::RelationshipFilter;
use serde_json::Value;

pub async fn handle_relationship_command(
    cmd: RelationshipCommands,
    ctx: &LocaiCliContext,
    output_format: &str,
) -> locai::Result<()> {
    match cmd {
        RelationshipCommands::Create(args) => {
            if args.bidirectional {
                ctx.memory_manager
                    .create_bidirectional_relationship(
                        &args.from,
                        &args.to,
                        &args.relationship_type,
                    )
                    .await?;
                println!(
                    "Bidirectional relationship created between '{}' and '{}'",
                    args.from, args.to
                );
            } else {
                ctx.memory_manager
                    .create_relationship(&args.from, &args.to, &args.relationship_type)
                    .await?;
                println!("Relationship created from '{}' to '{}'", args.from, args.to);
            }
        }

        RelationshipCommands::Get(args) => {
            match ctx.memory_manager.get_relationship(&args.id).await? {
                Some(relationship) => {
                    if output_format == "json" {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&relationship)
                                .unwrap_or_else(|_| "{}".to_string())
                        );
                    } else {
                        print_relationship(&relationship);
                    }
                }
                None => {
                    println!("Relationship with ID '{}' not found.", args.id);
                }
            }
        }

        RelationshipCommands::List(args) => {
            let filter = args.relationship_type.map(|rel_type| RelationshipFilter {
                relationship_type: Some(rel_type),
                ..Default::default()
            });

            let relationships = ctx
                .memory_manager
                .list_relationships(filter, Some(args.limit), None)
                .await?;

            if output_format == "json" {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&relationships)
                        .unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                print_relationship_list(&relationships);
            }
        }

        RelationshipCommands::Delete(args) => {
            match ctx.memory_manager.delete_relationship(&args.id).await? {
                true => println!("Relationship '{}' deleted successfully.", args.id),
                false => println!(
                    "Relationship '{}' not found or could not be deleted.",
                    args.id
                ),
            }
        }

        RelationshipCommands::Related(args) => {
            let memories = ctx
                .memory_manager
                .get_related_memories(&args.id, args.relationship_type.as_deref(), &args.direction)
                .await?;

            if output_format == "json" {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&memories).unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                print_memory_list(&memories);
            }
        }

        RelationshipCommands::Update(args) => {
            let mut relationship = ctx
                .memory_manager
                .get_relationship(&args.id)
                .await?
                .ok_or_else(|| {
                    LocaiError::Other(format!("Relationship '{}' not found", args.id))
                })?;

            if let Some(relationship_type) = args.relationship_type {
                relationship.relationship_type = relationship_type;
            }

            if let Some(properties_str) = args.properties {
                let properties: Value = serde_json::from_str(&properties_str)
                    .map_err(|e| LocaiError::Other(format!("Invalid JSON properties: {}", e)))?;
                relationship.properties = properties;
            }

            let updated = ctx.memory_manager.update_relationship(relationship).await?;

            if output_format == "json" {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&updated).unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                println!(
                    "{}",
                    format_success(&format!(
                        "Relationship '{}' updated successfully.",
                        args.id.color(CliColors::accent())
                    ))
                );
            }
        }
    }

    Ok(())
}
