//! Relationship type command handlers

use crate::commands::RelationshipTypeCommands;
use crate::context::LocaiCliContext;
use crate::output::*;
use colored::Colorize;
use locai::LocaiError;
use locai::relationships::RelationshipTypeDef;
use serde_json::Value;
use std::fs;

pub async fn handle_relationship_type_command(
    cmd: RelationshipTypeCommands,
    ctx: &LocaiCliContext,
    output_format: &str,
) -> locai::Result<()> {
    match cmd {
        RelationshipTypeCommands::List => {
            let types = ctx.relationship_type_registry.list().await;

            if output_format == "json" {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&types).unwrap_or_else(|_| "[]".to_string())
                );
            } else if types.is_empty() {
                println!("{}", format_info("No relationship types registered."));
            } else {
                println!(
                    "{}",
                    format_info(&format!("Found {} relationship types:", types.len()))
                );
                println!();
                println!(
                    "{:<30} {:<15} {:<12} {:<12} {}",
                    "Name".color(CliColors::muted()).bold(),
                    "Inverse".color(CliColors::muted()).bold(),
                    "Symmetric".color(CliColors::muted()).bold(),
                    "Transitive".color(CliColors::muted()).bold(),
                    "Created".color(CliColors::muted()).bold()
                );
                println!("{}", "─".repeat(100).color(CliColors::muted()));

                for type_def in types {
                    println!(
                        "{:<30} {:<15} {:<12} {:<12} {}",
                        type_def.name.color(CliColors::accent()),
                        type_def
                            .inverse
                            .as_deref()
                            .unwrap_or("-")
                            .color(CliColors::muted()),
                        if type_def.symmetric {
                            "Yes".color(CliColors::success())
                        } else {
                            "No".color(CliColors::muted())
                        },
                        if type_def.transitive {
                            "Yes".color(CliColors::success())
                        } else {
                            "No".color(CliColors::muted())
                        },
                        type_def
                            .created_at
                            .format("%Y-%m-%d")
                            .to_string()
                            .color(CliColors::muted())
                    );
                }
            }
        }

        RelationshipTypeCommands::Get(args) => {
            match ctx.relationship_type_registry.get(&args.name).await {
                Some(type_def) => {
                    if output_format == "json" {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&type_def)
                                .unwrap_or_else(|_| "{}".to_string())
                        );
                    } else {
                        println!(
                            "{}",
                            "━━━ Relationship Type Details ━━━"
                                .color(CliColors::accent())
                                .bold()
                        );
                        println!(
                            "{}: {}",
                            "Name".color(CliColors::muted()),
                            type_def.name.color(CliColors::accent()).bold()
                        );
                        if let Some(inverse) = &type_def.inverse {
                            println!(
                                "{}: {}",
                                "Inverse".color(CliColors::muted()),
                                inverse.color(CliColors::accent())
                            );
                        }
                        println!(
                            "{}: {}",
                            "Symmetric".color(CliColors::muted()),
                            if type_def.symmetric {
                                "Yes".color(CliColors::success())
                            } else {
                                "No".color(CliColors::muted())
                            }
                        );
                        println!(
                            "{}: {}",
                            "Transitive".color(CliColors::muted()),
                            if type_def.transitive {
                                "Yes".color(CliColors::success())
                            } else {
                                "No".color(CliColors::muted())
                            }
                        );
                        println!(
                            "{}: {}",
                            "Created".color(CliColors::muted()),
                            type_def
                                .created_at
                                .format("%Y-%m-%d %H:%M:%S UTC")
                                .to_string()
                                .color(CliColors::primary())
                        );
                    }
                }
                None => {
                    println!(
                        "{}",
                        format_warning(&format!(
                            "Relationship type '{}' not found.",
                            args.name.color(CliColors::accent())
                        ))
                    );
                }
            }
        }

        RelationshipTypeCommands::Register(args) => {
            let mut type_def = RelationshipTypeDef::new(args.name.clone())
                .map_err(|e| LocaiError::Other(e.to_string()))?;

            if let Some(inverse) = args.inverse {
                type_def = type_def.with_inverse(inverse);
            }

            if args.symmetric {
                type_def = type_def.symmetric();
            }

            if args.transitive {
                type_def = type_def.transitive();
            }

            if let Some(schema_path) = args.schema {
                let schema_content = fs::read_to_string(&schema_path)
                    .map_err(|e| LocaiError::Other(format!("Failed to read schema file: {}", e)))?;
                let schema: Value = serde_json::from_str(&schema_content)
                    .map_err(|e| LocaiError::Other(format!("Invalid JSON schema: {}", e)))?;
                type_def = type_def.with_metadata_schema(schema);
            }

            match ctx
                .relationship_type_registry
                .register(type_def.clone())
                .await
            {
                Ok(()) => {
                    if output_format == "json" {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&type_def)
                                .unwrap_or_else(|_| "{}".to_string())
                        );
                    } else {
                        println!(
                            "{}",
                            format_success(&format!(
                                "Relationship type '{}' registered successfully.",
                                args.name.color(CliColors::accent())
                            ))
                        );
                    }
                }
                Err(e) => {
                    output_error(
                        &format!("Failed to register relationship type: {}", e),
                        output_format,
                    );
                }
            }
        }

        RelationshipTypeCommands::Update(args) => {
            let mut type_def = ctx
                .relationship_type_registry
                .get(&args.name)
                .await
                .ok_or_else(|| {
                    LocaiError::Other(format!("Relationship type '{}' not found", args.name))
                })?;

            if let Some(inverse) = args.inverse {
                type_def = type_def.with_inverse(inverse);
            }

            if let Some(symmetric) = args.symmetric {
                if symmetric {
                    type_def = type_def.symmetric();
                } else {
                    let mut new_def = RelationshipTypeDef::new(type_def.name.clone())
                        .map_err(|e| LocaiError::Other(e.to_string()))?;
                    if let Some(inv) = &type_def.inverse {
                        new_def = new_def.with_inverse(inv.clone());
                    }
                    if type_def.transitive {
                        new_def = new_def.transitive();
                    }
                    if let Some(schema) = &type_def.metadata_schema {
                        new_def = new_def.with_metadata_schema(schema.clone());
                    }
                    type_def = new_def;
                }
            }

            if let Some(transitive) = args.transitive {
                if transitive {
                    type_def = type_def.transitive();
                } else {
                    let mut new_def = RelationshipTypeDef::new(type_def.name.clone())
                        .map_err(|e| LocaiError::Other(e.to_string()))?;
                    if let Some(inv) = &type_def.inverse {
                        new_def = new_def.with_inverse(inv.clone());
                    }
                    if type_def.symmetric {
                        new_def = new_def.symmetric();
                    }
                    if let Some(schema) = &type_def.metadata_schema {
                        new_def = new_def.with_metadata_schema(schema.clone());
                    }
                    type_def = new_def;
                }
            }

            if let Some(schema_path) = args.schema {
                let schema_content = fs::read_to_string(&schema_path)
                    .map_err(|e| LocaiError::Other(format!("Failed to read schema file: {}", e)))?;
                let schema: Value = serde_json::from_str(&schema_content)
                    .map_err(|e| LocaiError::Other(format!("Invalid JSON schema: {}", e)))?;
                type_def = type_def.with_metadata_schema(schema);
            }

            match ctx
                .relationship_type_registry
                .update(type_def.clone())
                .await
            {
                Ok(()) => {
                    if output_format == "json" {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&type_def)
                                .unwrap_or_else(|_| "{}".to_string())
                        );
                    } else {
                        println!(
                            "{}",
                            format_success(&format!(
                                "Relationship type '{}' updated successfully.",
                                args.name.color(CliColors::accent())
                            ))
                        );
                    }
                }
                Err(e) => {
                    output_error(
                        &format!("Failed to update relationship type: {}", e),
                        output_format,
                    );
                }
            }
        }

        RelationshipTypeCommands::Delete(args) => {
            match ctx.relationship_type_registry.delete(&args.name).await {
                Ok(()) => {
                    if output_format == "json" {
                        let result = serde_json::json!({
                            "success": true,
                            "name": args.name
                        });
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&result)
                                .unwrap_or_else(|_| "{}".to_string())
                        );
                    } else {
                        println!(
                            "{}",
                            format_success(&format!(
                                "Relationship type '{}' deleted successfully.",
                                args.name.color(CliColors::accent())
                            ))
                        );
                    }
                }
                Err(e) => {
                    output_error(
                        &format!("Failed to delete relationship type: {}", e),
                        output_format,
                    );
                }
            }
        }

        RelationshipTypeCommands::Metrics => {
            let types = ctx.relationship_type_registry.list().await;
            let count = types.len();
            let symmetric_count = types.iter().filter(|t| t.symmetric).count();
            let transitive_count = types.iter().filter(|t| t.transitive).count();
            let with_inverse_count = types.iter().filter(|t| t.inverse.is_some()).count();

            if output_format == "json" {
                let result = serde_json::json!({
                    "total_types": count,
                    "symmetric_types": symmetric_count,
                    "transitive_types": transitive_count,
                    "types_with_inverse": with_inverse_count
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                println!(
                    "{}",
                    "━━━ Relationship Type Metrics ━━━"
                        .color(CliColors::accent())
                        .bold()
                );
                println!(
                    "{}: {}",
                    "Total Types".color(CliColors::muted()),
                    count.to_string().color(CliColors::accent()).bold()
                );
                println!(
                    "{}: {} ({:.1}%)",
                    "Symmetric Types".color(CliColors::muted()),
                    symmetric_count.to_string().color(CliColors::success()),
                    if count > 0 {
                        (symmetric_count as f64 / count as f64) * 100.0
                    } else {
                        0.0
                    }
                );
                println!(
                    "{}: {} ({:.1}%)",
                    "Transitive Types".color(CliColors::muted()),
                    transitive_count.to_string().color(CliColors::success()),
                    if count > 0 {
                        (transitive_count as f64 / count as f64) * 100.0
                    } else {
                        0.0
                    }
                );
                println!(
                    "{}: {} ({:.1}%)",
                    "Types with Inverse".color(CliColors::muted()),
                    with_inverse_count.to_string().color(CliColors::info()),
                    if count > 0 {
                        (with_inverse_count as f64 / count as f64) * 100.0
                    } else {
                        0.0
                    }
                );
            }
        }

        RelationshipTypeCommands::Seed => {
            match ctx.relationship_type_registry.seed_common_types().await {
                Ok(()) => {
                    let types = ctx.relationship_type_registry.list().await;
                    if output_format == "json" {
                        let result = serde_json::json!({
                            "success": true,
                            "types_seeded": types.len()
                        });
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&result)
                                .unwrap_or_else(|_| "{}".to_string())
                        );
                    } else {
                        println!(
                            "{}",
                            format_success(&format!(
                                "Seeded {} common relationship types.",
                                types.len()
                            ))
                        );
                    }
                }
                Err(e) => {
                    output_error(
                        &format!("Failed to seed relationship types: {}", e),
                        output_format,
                    );
                }
            }
        }
    }

    Ok(())
}
