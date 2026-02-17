use anyhow::Result;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

pub struct Environment {
    vars: HashMap<String, String>,
    aliases: HashMap<String, String>,
    cwd: PathBuf,
    home_dir: PathBuf,
}

impl Environment {
    pub fn new() -> Result<Self> {
        let mut vars = HashMap::new();

        for (key, value) in env::vars() {
            vars.insert(key, value);
        }

        let home_dir = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;

        let piebash_home = home_dir.join(".piebash");
        std::fs::create_dir_all(&piebash_home)?;

        vars.insert("SHELL".to_string(), "piebash".to_string());
        vars.insert("PIEBASH_HOME".to_string(), piebash_home.to_string_lossy().to_string());

        let cwd = env::current_dir()?;

        Ok(Self {
            vars,
            aliases: HashMap::new(),
            cwd,
            home_dir,
        })
    }

    pub fn get_var(&self, key: &str) -> Option<String> {
        self.vars.get(key).cloned()
    }

    pub fn set_var(&mut self, key: &str, value: &str) {
        self.vars.insert(key.to_string(), value.to_string());
    }

    pub fn unset_var(&mut self, key: &str) {
        self.vars.remove(key);
    }

    pub fn get_all_vars(&self) -> &HashMap<String, String> {
        &self.vars
    }

    pub fn get_cwd(&self) -> &PathBuf {
        &self.cwd
    }

    pub fn set_cwd(&mut self, path: PathBuf) -> Result<()> {
        env::set_current_dir(&path)?;
        self.cwd = path;
        Ok(())
    }

    pub fn get_home_dir(&self) -> PathBuf {
        self.home_dir.clone()
    }

    pub fn set_alias(&mut self, name: String, value: String) {
        self.aliases.insert(name, value);
    }

    pub fn get_alias(&self, name: &str) -> Option<String> {
        self.aliases.get(name).cloned()
    }

    pub fn get_aliases(&self) -> &HashMap<String, String> {
        &self.aliases
    }

    pub fn remove_alias(&mut self, name: &str) {
        self.aliases.remove(name);
    }
}