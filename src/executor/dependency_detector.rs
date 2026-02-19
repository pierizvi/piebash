use regex::Regex;

#[derive(Debug, Clone)]
pub struct MissingDependency {
    pub language: String,
    pub package: String,
    pub package_manager: String,
    pub install_command: Vec<String>,
}

#[derive(Clone)]
pub struct DependencyDetector;

impl DependencyDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn parse_error(language: &str, stderr: &str, _stdout: &str) -> Option<Vec<MissingDependency>> {
        match language {
            "python" => Self::parse_python_error(stderr),
            "node" | "nodejs" => Self::parse_node_error(stderr),
            "ruby" => Self::parse_ruby_error(stderr),
            "go" => Self::parse_go_error(stderr),
            "rust" => Self::parse_rust_error(stderr),
            "java" => Self::parse_java_error(stderr),
            "php" => Self::parse_php_error(stderr),
            "perl" => Self::parse_perl_error(stderr),
            _ => None,
        }
    }

    // PYTHON
    fn parse_python_error(output: &str) -> Option<Vec<MissingDependency>> {
        let mut deps = Vec::new();

        // ModuleNotFoundError: No module named 'streamlit'
        let re1 = Regex::new(r"ModuleNotFoundError: No module named '([^']+)'").ok()?;
        for cap in re1.captures_iter(output) {
            if let Some(module) = cap.get(1) {
                let package = Self::python_import_to_package(module.as_str());
                deps.push(MissingDependency {
                    language: "python".to_string(),
                    package: package.clone(),
                    package_manager: "pip".to_string(),
                    install_command: vec!["install".to_string(), package],
                });
            }
        }

        // ImportError: cannot import name 'X' from 'Y'
        let re2 = Regex::new(r"ImportError:.*from '([^']+)'").ok()?;
        for cap in re2.captures_iter(output) {
            if let Some(module) = cap.get(1) {
                let package = Self::python_import_to_package(module.as_str());
                if !deps.iter().any(|d| d.package == package) {
                    deps.push(MissingDependency {
                        language: "python".to_string(),
                        package: package.clone(),
                        package_manager: "pip".to_string(),
                        install_command: vec!["install".to_string(), package],
                    });
                }
            }
        }

        if deps.is_empty() { None } else { Some(deps) }
    }

    // NODE.JS
    fn parse_node_error(output: &str) -> Option<Vec<MissingDependency>> {
        let mut deps = Vec::new();

        // Error: Cannot find module 'express'
        let re1 = Regex::new(r"Cannot find module '([^']+)'").ok()?;
        for cap in re1.captures_iter(output) {
            if let Some(module) = cap.get(1) {
                let module_name = module.as_str();
                if !Self::is_node_core_module(module_name) {
                    deps.push(MissingDependency {
                        language: "node".to_string(),
                        package: module_name.to_string(),
                        package_manager: "npm".to_string(),
                        install_command: vec!["install".to_string(), module_name.to_string()],
                    });
                }
            }
        }

        // Module not found: Error: Can't resolve 'react'
        let re2 = Regex::new(r"Can't resolve '([^']+)'").ok()?;
        for cap in re2.captures_iter(output) {
            if let Some(module) = cap.get(1) {
                let module_name = module.as_str();
                if !Self::is_node_core_module(module_name) && !deps.iter().any(|d| d.package == module_name) {
                    deps.push(MissingDependency {
                        language: "node".to_string(),
                        package: module_name.to_string(),
                        package_manager: "npm".to_string(),
                        install_command: vec!["install".to_string(), module_name.to_string()],
                    });
                }
            }
        }

        if deps.is_empty() { None } else { Some(deps) }
    }

    // RUBY
    fn parse_ruby_error(output: &str) -> Option<Vec<MissingDependency>> {
        let mut deps = Vec::new();

        // cannot load such file -- sinatra
        let re1 = Regex::new(r"cannot load such file -- ([^\s\(]+)").ok()?;
        for cap in re1.captures_iter(output) {
            if let Some(gem) = cap.get(1) {
                let gem_name = gem.as_str();
                deps.push(MissingDependency {
                    language: "ruby".to_string(),
                    package: gem_name.to_string(),
                    package_manager: "gem".to_string(),
                    install_command: vec!["install".to_string(), gem_name.to_string()],
                });
            }
        }

        if deps.is_empty() { None } else { Some(deps) }
    }

    // GO
    fn parse_go_error(output: &str) -> Option<Vec<MissingDependency>> {
        let mut deps = Vec::new();

        // package github.com/gin-gonic/gin is not in GOROOT
        let re1 = Regex::new(r"package ([^\s]+) is not in").ok()?;
        for cap in re1.captures_iter(output) {
            if let Some(pkg) = cap.get(1) {
                let pkg_name = pkg.as_str().to_string();
                deps.push(MissingDependency {
                    language: "go".to_string(),
                    package: pkg_name.clone(),
                    package_manager: "go".to_string(),
                    install_command: vec!["get".to_string(), pkg_name],
                });
            }
        }

        // no required module provides package github.com/...
        let re2 = Regex::new(r"no required module provides package ([^\s;]+)").ok()?;
        for cap in re2.captures_iter(output) {
            if let Some(pkg) = cap.get(1) {
                let pkg_name = pkg.as_str().to_string();
                if !deps.iter().any(|d| d.package == pkg_name) {
                    deps.push(MissingDependency {
                        language: "go".to_string(),
                        package: pkg_name.clone(),
                        package_manager: "go".to_string(),
                        install_command: vec!["get".to_string(), pkg_name],
                    });
                }
            }
        }

        if deps.is_empty() { None } else { Some(deps) }
    }

    // RUST
    fn parse_rust_error(output: &str) -> Option<Vec<MissingDependency>> {
        let mut deps = Vec::new();

        // error[E0432]: unresolved import `serde`
        let re1 = Regex::new(r"unresolved import `([^`]+)`").ok()?;
        for cap in re1.captures_iter(output) {
            if let Some(crate_name) = cap.get(1) {
                let name = crate_name.as_str().split("::").next().unwrap_or(crate_name.as_str());
                deps.push(MissingDependency {
                    language: "rust".to_string(),
                    package: name.to_string(),
                    package_manager: "cargo".to_string(),
                    install_command: vec!["add".to_string(), name.to_string()],
                });
            }
        }

        if deps.is_empty() { None } else { Some(deps) }
    }

    // JAVA
    fn parse_java_error(output: &str) -> Option<Vec<MissingDependency>> {
        let mut deps = Vec::new();

        // error: package com.google.gson does not exist
        let re1 = Regex::new(r"package ([a-z.]+) does not exist").ok()?;
        for cap in re1.captures_iter(output) {
            if let Some(package) = cap.get(1) {
                let pkg_name = package.as_str();
                let maven_pkg = Self::java_package_to_maven(pkg_name);
                deps.push(MissingDependency {
                    language: "java".to_string(),
                    package: maven_pkg.clone(),
                    package_manager: "maven".to_string(),
                    install_command: vec!["dependency:get".to_string(), format!("-Dartifact={}", maven_pkg)],
                });
            }
        }

        if deps.is_empty() { None } else { Some(deps) }
    }

    // PHP
    fn parse_php_error(output: &str) -> Option<Vec<MissingDependency>> {
        let mut deps = Vec::new();

        // Fatal error: Uncaught Error: Class 'Monolog\Logger' not found
        let re1 = Regex::new(r"Class '([^']+)' not found").ok()?;
        for cap in re1.captures_iter(output) {
            if let Some(class) = cap.get(1) {
                let class_name = class.as_str();
                let package = Self::php_class_to_package(class_name);
                deps.push(MissingDependency {
                    language: "php".to_string(),
                    package: package.clone(),
                    package_manager: "composer".to_string(),
                    install_command: vec!["require".to_string(), package],
                });
            }
        }

        if deps.is_empty() { None } else { Some(deps) }
    }

    // PERL
    fn parse_perl_error(output: &str) -> Option<Vec<MissingDependency>> {
        let mut deps = Vec::new();

        // Can't locate LWP/UserAgent.pm in @INC
        let re1 = Regex::new(r"Can't locate ([^\s]+)\.pm in").ok()?;
        for cap in re1.captures_iter(output) {
            if let Some(module) = cap.get(1) {
                let module_name = module.as_str().replace('/', "::").to_string();
                deps.push(MissingDependency {
                    language: "perl".to_string(),
                    package: module_name.clone(),
                    package_manager: "cpan".to_string(),
                    install_command: vec!["install".to_string(), module_name],
                });
            }
        }

        if deps.is_empty() { None } else { Some(deps) }
    }

    // HELPER: Python import to package mapping
    fn python_import_to_package(import_name: &str) -> String {
        let mapping = [
            ("PIL", "Pillow"),
            ("cv2", "opencv-python"),
            ("sklearn", "scikit-learn"),
            ("yaml", "PyYAML"),
            ("bs4", "beautifulsoup4"),
            ("dateutil", "python-dateutil"),
            ("dotenv", "python-dotenv"),
            ("sqlalchemy", "SQLAlchemy"),
            ("redis", "redis"),
            ("pymongo", "pymongo"),
            ("psycopg2", "psycopg2-binary"),
        ];

        for (import, package) in &mapping {
            if import_name == *import || import_name.starts_with(&format!("{}.", import)) {
                return package.to_string();
            }
        }

        import_name.split('.').next().unwrap_or(import_name).to_string()
    }

    // HELPER: Check if Node.js core module
    fn is_node_core_module(module: &str) -> bool {
        let core_modules = [
            "assert", "buffer", "child_process", "cluster", "crypto",
            "dgram", "dns", "domain", "events", "fs", "http", "https",
            "net", "os", "path", "punycode", "querystring", "readline",
            "repl", "stream", "string_decoder", "timers", "tls", "tty",
            "url", "util", "v8", "vm", "zlib",
        ];
        core_modules.contains(&module)
    }

    // HELPER: Java package to Maven coordinates
    fn java_package_to_maven(package: &str) -> String {
        let mappings = [
            ("com.google.gson", "com.google.code.gson:gson:2.10"),
            ("org.json", "org.json:json:20230227"),
            ("com.fasterxml.jackson", "com.fasterxml.jackson.core:jackson-databind:2.15.0"),
        ];

        for (pkg, maven) in &mappings {
            if package.starts_with(pkg) {
                return maven.to_string();
            }
        }

        format!("{}:{}:LATEST", package, package.split('.').last().unwrap_or("artifact"))
    }

    // HELPER: PHP class to Composer package
    fn php_class_to_package(class: &str) -> String {
        let mappings = [
            ("Monolog\\", "monolog/monolog"),
            ("Symfony\\", "symfony/symfony"),
            ("Guzzle\\", "guzzlehttp/guzzle"),
            ("PHPUnit\\", "phpunit/phpunit"),
        ];

        for (namespace, package) in &mappings {
            if class.starts_with(namespace) {
                return package.to_string();
            }
        }

        class.split('\\').next().unwrap_or(class).to_lowercase()
    }
}