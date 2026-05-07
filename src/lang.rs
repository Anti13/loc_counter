use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Language {
    C,
    Cpp,
    CSharp,
    Go,
    Java,
    JavaScript,
    Php,
    Python,
    Ruby,
    Rust,
    Shell,
    TypeScript,
}

impl Language {
    pub fn name(self) -> &'static str {
        match self {
            Language::C => "C",
            Language::Cpp => "C++",
            Language::CSharp => "C#",
            Language::Go => "Go",
            Language::Java => "Java",
            Language::JavaScript => "JavaScript",
            Language::Php => "PHP",
            Language::Python => "Python",
            Language::Ruby => "Ruby",
            Language::Rust => "Rust",
            Language::Shell => "Shell",
            Language::TypeScript => "TypeScript",
        }
    }
}

pub fn from_path(path: &Path) -> Option<Language> {
    let ext = path.extension()?.to_str()?;
    let lower = ext.to_ascii_lowercase();
    Some(match lower.as_str() {
        "rs" => Language::Rust,
        "c" | "h" => Language::C,
        "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" => Language::Cpp,
        "cs" => Language::CSharp,
        "go" => Language::Go,
        "java" => Language::Java,
        "js" | "mjs" | "cjs" => Language::JavaScript,
        "php" => Language::Php,
        "py" => Language::Python,
        "rb" => Language::Ruby,
        "sh" | "bash" => Language::Shell,
        "ts" | "tsx" => Language::TypeScript,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_common_extensions() {
        assert_eq!(from_path(Path::new("foo.rs")), Some(Language::Rust));
        assert_eq!(from_path(Path::new("a/b/c.cpp")), Some(Language::Cpp));
        assert_eq!(from_path(Path::new("X.HPP")), Some(Language::Cpp));
        assert_eq!(from_path(Path::new("script.sh")), Some(Language::Shell));
    }

    #[test]
    fn unknown_extensions_are_none() {
        assert_eq!(from_path(Path::new("readme.md")), None);
        assert_eq!(from_path(Path::new("Makefile")), None);
        assert_eq!(from_path(Path::new("noext")), None);
    }

    #[test]
    fn multiple_dots_use_final_extension() {
        assert_eq!(from_path(Path::new("module.test.rs")), Some(Language::Rust));
        assert_eq!(from_path(Path::new("foo.bar.cpp")), Some(Language::Cpp));
    }

    #[test]
    fn dotfiles_without_extension_are_none() {
        // `.bashrc` has no extension as far as `Path::extension` is concerned.
        assert_eq!(from_path(Path::new(".bashrc")), None);
        assert_eq!(from_path(Path::new(".gitignore")), None);
    }

    #[test]
    fn empty_extension_is_none() {
        assert_eq!(from_path(Path::new("foo.")), None);
    }

    #[test]
    fn every_supported_language_is_detected() {
        let cases: &[(&str, Language)] = &[
            ("a.rs", Language::Rust),
            ("a.c", Language::C),
            ("a.h", Language::C),
            ("a.cpp", Language::Cpp),
            ("a.cc", Language::Cpp),
            ("a.cxx", Language::Cpp),
            ("a.hpp", Language::Cpp),
            ("a.cs", Language::CSharp),
            ("a.go", Language::Go),
            ("a.java", Language::Java),
            ("a.js", Language::JavaScript),
            ("a.mjs", Language::JavaScript),
            ("a.cjs", Language::JavaScript),
            ("a.php", Language::Php),
            ("a.py", Language::Python),
            ("a.rb", Language::Ruby),
            ("a.sh", Language::Shell),
            ("a.bash", Language::Shell),
            ("a.ts", Language::TypeScript),
            ("a.tsx", Language::TypeScript),
        ];
        for (filename, expected) in cases {
            assert_eq!(
                from_path(Path::new(filename)),
                Some(*expected),
                "language detection failed for {filename}"
            );
        }
    }
}
