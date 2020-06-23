use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::string::ToString;

#[derive(Clone, Serialize, Deserialize)]
pub enum Exclusion {
    Directory(PathBuf),
    Pattern(String),
}

impl ToString for Exclusion {
    fn to_string(&self) -> String {
        match self {
            Self::Directory(dir) => dir.display().to_string(),
            Self::Pattern(pattern) => pattern.clone(),
        }
    }
}

lazy_static! {
    pub static ref DEFAULT_EXCLUDE_PATTERNS: [Exclusion; 12] = [
        Exclusion::Directory("/lost+found".into()),
        Exclusion::Directory("/dev".into()),
        Exclusion::Directory("/proc".into()),
        Exclusion::Directory("/sys".into()),
        Exclusion::Directory("/tmp".into()),
        Exclusion::Pattern("*/.svn".into()),
        Exclusion::Pattern("*/CVS".into()),
        Exclusion::Pattern("*/.git".into()),
        Exclusion::Pattern("*/.hg".into()),
        Exclusion::Pattern("*/.bzr".into()),
        Exclusion::Pattern("*/node_modules".into()),
        Exclusion::Pattern("*/target".into()),
    ];
}
