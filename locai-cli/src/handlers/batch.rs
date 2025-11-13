//! Batch command handlers

use crate::commands::BatchCommands;
use crate::context::LocaiCliContext;
use crate::output::*;
use atty;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use locai::LocaiError;
use locai::batch::{BatchExecutor, BatchExecutorConfig, BatchOperation, BatchResult};
use std::fs;

pub async fn handle_batch_command(
    cmd: BatchCommands,
    ctx: &LocaiCliContext,
    output_format: &str,
) -> locai::Result<()> {
    match cmd {
        BatchCommands::Execute(args) => {
            let file_contents = fs::read_to_string(&args.file)
                .map_err(|e| LocaiError::Other(format!("Failed to read batch file: {}", e)))?;

            let (operations, file_transaction): (Vec<BatchOperation>, Option<bool>) =
                if let Ok(ops_array) =
                    serde_json::from_str::<Vec<serde_json::Value>>(&file_contents)
                {
                    (
                        ops_array
                            .into_iter()
                            .filter_map(|v| serde_json::from_value::<BatchOperation>(v).ok())
                            .collect(),
                        None,
                    )
                } else if let Ok(batch_obj) =
                    serde_json::from_str::<serde_json::Value>(&file_contents)
                {
                    if let Some(ops_array) = batch_obj.get("operations").and_then(|v| v.as_array())
                    {
                        let file_transaction =
                            batch_obj.get("transaction").and_then(|v| v.as_bool());
                        (
                            ops_array
                                .iter()
                                .filter_map(|v| {
                                    serde_json::from_value::<BatchOperation>(v.clone()).ok()
                                })
                                .collect::<Vec<_>>(),
                            file_transaction,
                        )
                    } else {
                        return Err(LocaiError::Other(
                        "Batch file must contain 'operations' array or be an array of operations"
                            .to_string(),
                    ));
                    }
                } else {
                    return Err(LocaiError::Other(
                        "Invalid JSON format in batch file".to_string(),
                    ));
                };

            if operations.is_empty() {
                return Err(LocaiError::Other(
                    "No operations found in batch file".to_string(),
                ));
            }

            let transaction = args.transaction || file_transaction.unwrap_or(false);

            // Create progress bar if stdout is a TTY and not JSON output
            let pb = if atty::is(atty::Stream::Stdout)
                && output_format != "json"
                && operations.len() > 5
            {
                let pb = ProgressBar::new(operations.len() as u64);
                pb.set_style(
                    ProgressStyle::default_bar()
                        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                        .unwrap()
                        .progress_chars("#>-")
                );
                Some(pb)
            } else {
                None
            };

            let storage = ctx.memory_manager.storage().clone();
            let config = BatchExecutorConfig::default();
            let executor = BatchExecutor::new(storage, config);

            // Execute operations with progress tracking
            let response = if let Some(ref progress_bar) = pb {
                // For now, we'll update progress after execution
                // In a more advanced implementation, we could hook into the executor
                let response = executor
                    .execute(operations.clone(), transaction)
                    .await
                    .map_err(|e| LocaiError::Other(format!("Batch execution failed: {}", e)))?;

                progress_bar
                    .finish_with_message(format!("Completed {} operations", response.completed));
                response
            } else {
                executor
                    .execute(operations.clone(), transaction)
                    .await
                    .map_err(|e| LocaiError::Other(format!("Batch execution failed: {}", e)))?
            };

            if output_format == "json" {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&response).unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                println!(
                    "{}",
                    format_info(&format!(
                        "Batch execution completed: {} succeeded, {} failed",
                        response.completed, response.failed
                    ))
                );

                if response.has_errors() {
                    println!("\nErrors:");
                    for result in &response.results {
                        if let BatchResult::Error {
                            operation_index,
                            error,
                            ..
                        } = result
                        {
                            println!(
                                "  Operation {}: {}",
                                operation_index.to_string().color(CliColors::error()),
                                error.color(CliColors::error())
                            );
                        }
                    }
                }

                if response.completed > 0 {
                    println!("\nSuccessful operations:");
                    for result in &response.results {
                        if let BatchResult::Success {
                            operation_index,
                            resource_id,
                            ..
                        } = result
                        {
                            println!(
                                "  Operation {}: {}",
                                operation_index.to_string().color(CliColors::success()),
                                resource_id.color(CliColors::accent())
                            );
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
