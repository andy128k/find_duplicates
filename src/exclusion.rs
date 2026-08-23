use std::{fmt, path::PathBuf, sync::LazyLock};

#[derive(Clone)]
pub enum Exclusion {
    Directory(PathBuf),
    Pattern(String),
}

impl fmt::Display for Exclusion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Directory(dir) => write!(f, "{}", dir.display()),
            Self::Pattern(pattern) => write!(f, "{}", pattern),
        }
    }
}

pub static DEFAULT_EXCLUDE_PATTERNS: LazyLock<[Exclusion; 12]> = LazyLock::new(|| {
    [
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
    ]
});
