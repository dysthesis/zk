use std::{fs, path::PathBuf};

use crate::template::Template;

#[derive(Debug)]
pub enum Subcommand {
    Inspect(Option<PathBuf>),
    Links(PathBuf),
    Backlinks(PathBuf),
    Query(String),
    Search(String),
    List,
    New { template: Template, path: String },
}

/// Parsed ommand-line arguments
#[derive(Debug)]
pub struct Args {
    pub subcommand: Subcommand,
    /// Whether to output the results as json
    pub json: bool,
    pub vault_dir: PathBuf,
}

impl Args {
    /// Parse the arguments from the command line
    pub fn parse() -> Result<Args, lexopt::Error> {
        use lexopt::prelude::*;

        let mut subcommand = None;
        let mut argument = None;
        let mut parser = lexopt::Parser::from_env();
        let mut json = false;
        let mut vault_dir = std::env::current_dir().unwrap();
        let mut variables = None;
        let mut template_file = None;
        while let Some(arg) = parser.next()? {
            match arg {
                Value(val) if subcommand.is_none() => {
                    subcommand = Some(val.clone().string()?);
                }
                Value(val) => {
                    argument = Some(val.string()?);
                }
                Short('j') | Long("json") => {
                    json = true;
                }
                Short('d') | Long("vault-dir") => {
                    let path = parser.value()?.parse::<String>()?.to_string();
                    vault_dir = PathBuf::from(path);
                }
                Short('v') | Long("variables") => {
                    variables = Some(parser.value()?.parse::<String>()?.to_string());
                }
                Short('t') | Long("template-file") => {
                    template_file = Some(parser.value()?.parse::<String>()?.to_string());
                }
                Short('h') | Long("help") => {
                    let target: Option<String> = parser
                        .value()
                        .ok()
                        .map_or_else(|| None, |res| res.parse::<String>().ok());
                    let help_text = match target {
                        Some(val) if val == "subcommands" => {
                            "Available subcommmands are: inspect, links, backlinks"
                        }
                        _ => {
                            "Usage: n [-j|--json] [-d|--vault-dir=DIR] SUBCOMMAND PATH\n\nTo see the available subcommands, run zk --help subcommands."
                        }
                    };
                    println!("{help_text}");
                    std::process::exit(0);
                }
                _ => return Err(arg.unexpected()),
            }
        }
        let subcommand = match subcommand.ok_or("missing subcommand")? {
            val if val == "inspect" => {
                Subcommand::Inspect(argument.map_or_else(|| None, |val| Some(PathBuf::from(val))))
            }
            val if val == "query" => Subcommand::Query(argument.ok_or("missing argument")?),
            val if val == "search" => Subcommand::Search(argument.ok_or("missing argument")?),
            val if (val == "list") || (val == "ls") => Subcommand::List,
            val if val == "backlinks" => {
                Subcommand::Backlinks(argument.ok_or("missing argument")?.into())
            }
            val if val == "links" => Subcommand::Links(argument.ok_or("missing argument")?.into()),
            val if val == "new" => {
                let template =
                    fs::read_to_string::<String>(template_file.ok_or("missing argument")?).unwrap();
                let path = argument.ok_or("missing argument")?.into();
                let template = Template::new(template, variables);
                Subcommand::New { template, path }
            }
            _ => todo!(),
        };

        Ok(Args {
            subcommand,
            json,
            vault_dir,
        })
    }
}
