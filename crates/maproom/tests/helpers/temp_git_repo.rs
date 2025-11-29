use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Helper for creating temporary git repositories in tests.
pub struct TempGitRepo {
    dir: TempDir,
}

impl TempGitRepo {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().expect("failed to create temp dir");

        // Initialize git repo
        Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .expect("failed to init git repo");

        // Configure git user for commits
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(dir.path())
            .output()
            .expect("failed to configure git");

        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(dir.path())
            .output()
            .expect("failed to configure git");

        Self { dir }
    }

    pub fn path(&self) -> PathBuf {
        self.dir.path().to_path_buf()
    }

    pub fn create_file(&self, name: &str, content: &str) {
        let path = self.dir.path().join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, content).unwrap();
    }

    pub fn create_and_commit_file(&self, name: &str, content: &str) {
        self.create_file(name, content);
        Command::new("git")
            .args(["add", name])
            .current_dir(self.dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", &format!("add {}", name)])
            .current_dir(self.dir.path())
            .output()
            .unwrap();
    }

    pub fn modify_file(&self, name: &str, content: &str) {
        std::fs::write(self.dir.path().join(name), content).unwrap();
    }

    pub fn delete_file(&self, name: &str) {
        std::fs::remove_file(self.dir.path().join(name)).unwrap();
    }

    #[allow(dead_code)]
    pub fn rename_file(&self, old: &str, new: &str) {
        std::fs::rename(self.dir.path().join(old), self.dir.path().join(new)).unwrap();
    }

    pub fn create_git_lock(&self) {
        std::fs::write(self.dir.path().join(".git/index.lock"), "lock").unwrap();
    }

    pub fn remove_git_lock(&self) {
        let _ = std::fs::remove_file(self.dir.path().join(".git/index.lock"));
    }

    #[allow(dead_code)]
    pub fn stage_file(&self, name: &str) {
        Command::new("git")
            .args(["add", name])
            .current_dir(self.dir.path())
            .output()
            .unwrap();
    }

    pub fn stage_rename(&self, old: &str, new: &str) {
        Command::new("git")
            .args(["mv", old, new])
            .current_dir(self.dir.path())
            .output()
            .unwrap();
    }

    /// Stage a file for commit.
    pub fn stage(&self, name: &str) {
        Command::new("git")
            .args(["add", name])
            .current_dir(self.dir.path())
            .output()
            .unwrap();
    }

    /// Commit staged changes with the given message.
    pub fn commit(&self, message: &str) {
        Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(self.dir.path())
            .output()
            .unwrap();
    }
}

impl Default for TempGitRepo {
    fn default() -> Self {
        Self::new()
    }
}
