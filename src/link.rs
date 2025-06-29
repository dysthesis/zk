use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

use owo_colors::OwoColorize;
use percent_encoding::percent_decode_str;
use serde::Serialize;

use crate::path::MarkdownPath;

#[derive(Debug, Serialize, Clone, Hash, PartialEq, Eq)]
/// A link in a Markdown file
pub struct Link {
    pub text: String,
    pub url: String,
}

impl Link {
    /// Check if the link points to the given Markdown document
    pub fn points_to(&self, target: &MarkdownPath) -> bool {
        if let Some(path) = self.to_markdown_path(
            target
                .path()
                .parent()
                .unwrap_or_else(|| Path::new(""))
                .to_path_buf(),
        ) {
            return &path == target;
        }
        false
    }

    #[inline]
    pub fn to_markdown_path(&self, base_path: PathBuf) -> Option<MarkdownPath> {
        if let Err(url::ParseError::RelativeUrlWithoutBase) = url::Url::parse(self.url.as_str()) {
            MarkdownPath::new(base_path, PathBuf::from(self.url.clone())).ok()
        } else {
            None
        }
    }
}

impl Display for Link {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let url = percent_decode_str(self.url.as_ref())
            .decode_utf8_lossy()
            .to_string();
        write!(f, "{}", url.bright_blue().underline())
    }
}
