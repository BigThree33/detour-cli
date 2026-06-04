use anyhow::Result;
use clap::Parser;
use detour_cli::capture::capture_from_args;
use detour_cli::cli::{Cli, Commands, HookCommands, SkillCommands};

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
            println!("detour list");
            println!("root: {}", args.root.display());
            println!("json: {}", args.json);
        }
        Commands::Show(args) => {
            println!("detour show");
            println!("id_or_path: {}", args.id_or_path);
            println!("json: {}", args.json);
        }
        Commands::Search(args) => {
            println!("detour search");
            println!("query: {}", args.query);
            println!("root: {}", args.root.display());
            println!("json: {}", args.json);
        }
        Commands::Hook(args) => match args.command {
            HookCommands::Claude(claude) => {
                println!("detour hook claude");
                println!("command: {:?}", claude.command);
            }
        },
        Commands::Skill(args) => match args.command {
            SkillCommands::Create(command) => {
                println!("detour skill create");
                println!("target: {:?}", command.target);
                if let Some(dir) = command.dir {
                    println!("dir: {}", dir.display());
                }
                println!("force: {}", command.force);
            }
            SkillCommands::Install(command) => {
                println!("detour skill install");
                println!("target: {:?}", command.target);
                if let Some(dir) = command.dir {
                    println!("dir: {}", dir.display());
                }
                println!("force: {}", command.force);
            }
            SkillCommands::Print(command) => {
                println!("detour skill print");
                println!("target: {:?}", command.target);
            }
            SkillCommands::Path(command) => {
                println!("detour skill path");
                println!("target: {:?}", command.target);
            }
        },
    }

    Ok(())
}
