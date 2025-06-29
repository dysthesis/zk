mod cli;
mod document;
mod link;
mod path;
mod query;
mod rank;
mod search;
mod template;
mod vault;

use std::collections::HashMap;

use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use serde::Serialize;

use crate::{
    cli::{Args, Subcommand},
    document::Document,
    path::MarkdownPath,
    query::Query,
    rank::rank,
    vault::Vault,
};

pub const MAX_RESULTS: usize = 10;

fn main() {
    let args = Args::parse().unwrap();
    let vault = Vault::new(args.vault_dir.clone()).unwrap();
    const MAX_ITER: usize = 100_000;
    const TOLERANCE: f32 = 0.0000001;
    // TODO: Pretty-print the results
    match args.subcommand {
        Subcommand::New { template, path } => {
            let path = vault.path().join(format!("{path}.md"));
            template.write(&path).unwrap();
            println!("{}", path.to_string_lossy());
        }
        Subcommand::Search(query) => {
            let bm25: Vec<(Document, f32)> = vault
                .search(query)
                .into_par_iter()
                // We don't care about documents with no matches.
                .filter(|(_, score)| score > &0f32)
                .collect();
            let matches: Vec<&Document> = bm25.iter().map(|(doc, _)| doc).collect();

            let rank: HashMap<Document, f32> = matches
                .iter()
                .zip(rank(matches.clone(), vault.path(), MAX_ITER, TOLERANCE))
                .map(|(k, v)| ((**k).clone(), v))
                .collect();

            // How much should the BM25 score count over the PageRank score?
            let factor = 0.7f32;

            #[derive(Serialize)]
            /// Label the results in the JSON output
            struct SearchResult {
                document: Document,
                bm25: f32,
                rank: f32,
                combined: f32,
            }

            // Adjust the score to incorporate the pagerank score
            let mut res: Vec<SearchResult> = bm25
                .into_iter()
                .map(|(doc, bm25)| {
                    let rank = rank.get(&doc).unwrap();
                    SearchResult {
                        document: doc.clone(),
                        bm25,
                        rank: rank.to_owned(),
                        combined: (factor * bm25) + ((1f32 - factor) * rank),
                    }
                })
                .collect();

            res.sort_unstable_by(|a, b| {
                b.combined
                    .partial_cmp(&a.combined)
                    .unwrap_or(std::cmp::Ordering::Greater)
            });
            res.truncate(MAX_RESULTS);
            if args.json {
                println!("{}", serde_json::to_string(&res).unwrap());
            } else {
                let res: Vec<(String, f32, f32, f32)> = res
                    .into_iter()
                    .map(|result| {
                        (
                            result
                                .document
                                .get_metadata(&"title".to_string())
                                .map_or_else(|| "".to_string(), |res| res.to_string()),
                            result.bm25,
                            result.rank,
                            result.combined,
                        )
                    })
                    .collect();
                let mut builder = tabled::builder::Builder::new();
                builder.push_record(["Title", "BM25", "Rank", "Score"]);
                res.iter().for_each(|(title, bm25, rank, combined)| {
                    builder.push_record([
                        title,
                        &bm25.to_string(),
                        &rank.to_string(),
                        &combined.to_string(),
                    ])
                });
                let mut table = builder.build();
                table.with(tabled::settings::style::Style::rounded());
                println!("{table}");
            }
        }
        Subcommand::Query(query) => {
            let parsed_query = Query::parse(query.as_str()).unwrap();
            let results = vault.query(parsed_query);
            results
                .par_iter()
                .filter_map(|doc| doc.get_metadata(&"title".to_string()))
                .for_each(|title| println!("{title}"));
        }
        Subcommand::Inspect(path) => {
            let base_path = args.vault_dir;

            match path {
                Some(path) => {
                    let full_path = MarkdownPath::new(base_path, path).unwrap();
                    let document = vault.get_document(&full_path).unwrap();
                    if args.json {
                        println!("{}", serde_json::to_string(document).unwrap());
                    } else {
                        println!("{document}");
                    }
                }
                // Print out the whole vault if no arguments are provided
                None => {
                    if args.json {
                        println!("{}", serde_json::to_string(&vault).unwrap());
                    } else {
                        println!("{vault}");
                    }
                }
            }
        }
        Subcommand::Backlinks(path) => {
            let base_path = args.vault_dir;
            let full_path = MarkdownPath::new(base_path, path).unwrap();
            let backlinks = vault.find_backlinks(&full_path);
            if args.json {
                println!("{}", serde_json::to_string(&backlinks).unwrap());
            } else {
                let formatted_links: Vec<String> = backlinks
                    .into_par_iter()
                    .map(|val| val.to_string())
                    .collect();

                let mut formatted_links = tabled::Table::new(formatted_links);
                formatted_links.with(tabled::settings::style::Style::rounded());

                println!("{formatted_links}");
            }
        }
        Subcommand::Links(path) => {
            let base_path = args.vault_dir;
            let full_path = MarkdownPath::new(base_path, path).unwrap();
            let document = vault.get_document(&full_path).unwrap();
            let links = document.links();
            if args.json {
                println!("{}", serde_json::to_string(&links).unwrap());
            } else {
                println!("{links:?}");
            }
        }
        Subcommand::List => {
            let mut res: Vec<(Document, f32)> = vault
                .documents()
                .into_iter()
                .zip(rank(vault.documents(), vault.path(), MAX_ITER, TOLERANCE))
                .map(|(k, v)| (k.to_owned(), v))
                .collect();
            res.sort_unstable_by(|a, b| {
                b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Greater)
            });

            if args.json {
                println!("{}", serde_json::to_string(&res).unwrap());
            } else {
                let res: Vec<(String, f32)> = res
                    .into_iter()
                    .map(|(k, v)| {
                        (
                            k.get_metadata(&"title".to_string())
                                .map_or_else(|| "".to_string(), |res| res.to_string()),
                            v,
                        )
                    })
                    .collect();
                let mut builder = tabled::builder::Builder::new();
                builder.push_record(["Title", "Score"]);
                res.iter()
                    .for_each(|(k, v)| builder.push_record([k, &v.to_string()]));
                let mut table = builder.build();
                table.with(tabled::settings::style::Style::rounded());
                println!("{table}");
            }
        }
    }
}
