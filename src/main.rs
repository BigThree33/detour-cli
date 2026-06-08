use std::io::Read;

use anyhow::Result;
use clap::Parser;
use detour_cli::capture::capture_from_args;
use detour_cli::claude::{
    detect_claude_project, install_claude_project, precompact_config_snippet, run_precompact,
    uninstall_claude_project,
};
use detour_cli::cli::{
    ClaudeHookCommands, ClaudeHookEvent, Cli, Commands, HookCommands, SkillCommands,
};
use detour_cli::ledger::{
    list_documents, recent_documents, recent_rules, search_documents, show_document,
};
use detour_cli::llm::{capture_schema, usage_prompt};
use detour_cli::skills::{default_skill_path, skill_content, write_skill};

// 解析命令行参数，并把每个子命令分发到对应处理逻辑。
fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init(args) => {
            println!("detour init");
            println!("root: {}", args.root.display());
            println!("force: {}", args.force);
        }
        Commands::Capture(args) => {
            let json = args.json;
            let result = capture_from_args(args)?;

            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!("detour capture");
                println!("source: {}", result.source);
                println!("dry_run: {}", result.dry_run);
                println!("output_dir: {}", result.output_dir);
                println!("documents: {}", result.document_count);

                if result.dry_run {
                    for document in result.documents {
                        println!(
                            "- {} ({} mistakes, not written)",
                            document.title,
                            document.mistakes.len()
                        );
                    }
                } else {
                    for saved_document in result.saved_documents {
                        println!(
                            "- {} ({} mistakes) -> {}",
                            saved_document.title, saved_document.mistake_count, saved_document.path
                        );
                    }
                }
            }
        }
        Commands::List(args) => {
            let documents = list_documents(&args.root)?;

            if args.json {
                println!("{}", serde_json::to_string_pretty(&documents)?);
            } else if documents.is_empty() {
                println!("no mistake documents found");
            } else {
                for document in documents {
                    println!("{} | {} | {}", document.id, document.title, document.path);
                }
            }
        }
        Commands::Show(args) => {
            let document = show_document(&args.root, &args.id_or_path)?;

            if args.json {
                println!("{}", serde_json::to_string_pretty(&document)?);
            } else {
                print!("{}", document.content);
            }
        }
        Commands::Search(args) => {
            let hits = search_documents(&args.root, &args.query)?;

            if args.json {
                println!("{}", serde_json::to_string_pretty(&hits)?);
            } else if hits.is_empty() {
                println!("no matches found");
            } else {
                for hit in hits {
                    println!("{} | {} | {}", hit.id, hit.title, hit.path);
                    println!("  {}", hit.snippet);
                }
            }
        }
        Commands::Recent(args) => {
            let documents = recent_documents(&args.root, args.limit)?;

            if args.json {
                println!("{}", serde_json::to_string_pretty(&documents)?);
            } else if documents.is_empty() {
                println!("no recent mistake documents found");
            } else {
                for document in documents {
                    println!("{} | {} | {}", document.id, document.title, document.path);
                }
            }
        }
        Commands::Rules(args) => {
            let rules = recent_rules(&args.root, args.limit)?;

            if args.json {
                println!("{}", serde_json::to_string_pretty(&rules)?);
            } else if rules.is_empty() {
                println!("no prevention rules found");
            } else {
                for rule in rules {
                    println!("- {} ({})", rule.rule, rule.title);
                }
            }
        }
        Commands::Schema => {
            println!("{}", serde_json::to_string_pretty(&capture_schema())?);
        }
        Commands::Prompt(args) => {
            print!("{}", usage_prompt(args.target));
        }
        Commands::Hook(args) => match args.command {
            HookCommands::Claude(claude) => match claude.command {
                ClaudeHookCommands::Detect(command) | ClaudeHookCommands::Status(command) => {
                    let status = detect_claude_project(std::path::Path::new("."));

                    if command.json {
                        println!("{}", serde_json::to_string_pretty(&status)?);
                    } else {
                        println!("Claude Code project integration");
                        println!("project_root: {}", status.project_root);
                        println!(
                            "precompact hook: {} ({})",
                            status.settings_path,
                            installed_label(status.precompact_installed)
                        );
                        println!(
                            "skill: {} ({})",
                            status.skill_path,
                            installed_label(status.skill_installed)
                        );
                        println!(
                            "capture command: {} ({})",
                            status.capture_command_path,
                            installed_label(status.capture_command_installed)
                        );
                        println!(
                            "rules command: {} ({})",
                            status.rules_command_path,
                            installed_label(status.rules_command_installed)
                        );
                    }
                }
                ClaudeHookCommands::Install(command) => {
                    let result = install_claude_project(
                        std::path::Path::new("."),
                        command.mode,
                        command.global,
                        command.force,
                    )?;

                    println!("Claude Code integration installed");
                    println!("mode: {}", result.mode);
                    for file in result.files {
                        println!("- {} (overwritten: {})", file.path, file.overwritten);
                    }
                }
                ClaudeHookCommands::PrintConfig(command) => match command.event {
                    ClaudeHookEvent::PreCompact => {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&precompact_config_snippet())?
                        );
                    }
                },
                ClaudeHookCommands::Uninstall(command) => {
                    let result = uninstall_claude_project(
                        std::path::Path::new("."),
                        command.mode,
                        command.global,
                    )?;

                    println!("Claude Code integration uninstalled");
                    println!("mode: {}", result.mode);
                    for file in result.files {
                        println!("- {} (removed: {})", file.path, file.removed);
                    }
                }
                ClaudeHookCommands::RunPrecompact(command) => {
                    let mut hook_stdin = String::new();
                    std::io::stdin().read_to_string(&mut hook_stdin)?;
                    let result =
                        run_precompact(&hook_stdin, command.max_docs, command.out, command.dry_run);

                    if command.json {
                        println!("{}", serde_json::to_string_pretty(&result)?);
                    } else {
                        println!("Claude Code PreCompact capture");
                        println!("ok: {}", result.ok);
                        println!("documents: {}", result.document_count);
                        println!("output_dir: {}", result.output_dir);

                        for saved_document in result.saved_documents {
                            println!(
                                "- {} ({} mistakes) -> {}",
                                saved_document.title,
                                saved_document.mistake_count,
                                saved_document.path
                            );
                        }

                        for warning in result.warnings {
                            println!("warning: {warning}");
                        }
                    }
                }
            },
        },
        Commands::Skill(args) => match args.command {
            SkillCommands::Create(command) => {
                let result = write_skill(command.target, command.dir, command.force)?;
                println!("skill written: {}", result.path);
                println!("target: {}", result.target);
                println!("overwritten: {}", result.overwritten);
            }
            SkillCommands::Install(command) => {
                let result = write_skill(command.target, command.dir, command.force)?;
                println!("skill installed: {}", result.path);
                println!("target: {}", result.target);
                println!("overwritten: {}", result.overwritten);
            }
            SkillCommands::Print(command) => {
                print!("{}", skill_content(command.target));
            }
            SkillCommands::Path(command) => {
                println!("{}", default_skill_path(command.target).display());
            }
        },
    }

    Ok(())
}

// 把布尔状态转换成终端中更容易读的安装状态。
fn installed_label(installed: bool) -> &'static str {
    if installed {
        "installed"
    } else {
        "missing"
    }
}
