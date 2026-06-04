use anyhow::Result;
use clap::Parser;
use detour_cli::cli::{CaptureFormat, Cli, Commands, HookCommands, SkillCommands};

// Parse CLI arguments and dispatch each subcommand to its current placeholder handler.
fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init(args) => {
            println!("detour init");
            println!("root: {}", args.root.display());
            println!("force: {}", args.force);
        }
        Commands::Capture(args) => {
            println!("detour capture");

            if let Some(path) = args.from {
                println!("from: {}", path.display());
            }

            println!("stdin: {}", args.stdin);

            if let Some(out) = args.out {
                println!("out: {}", out.display());
            }

            println!("max_docs: {}", args.max_docs);
            println!("format: {}", format_name(args.format));
            println!("dry_run: {}", args.dry_run);
            println!("json: {}", args.json);
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

// Convert the capture format enum into the short name printed by placeholder output.
fn format_name(format: CaptureFormat) -> &'static str {
    match format {
        CaptureFormat::Markdown => "md",
        CaptureFormat::Json => "json",
    }
}
