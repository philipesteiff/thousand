use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use regex::Regex;

pub fn repo_root() -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("failed to run git rev-parse")?;
    if !output.status.success() {
        bail!("git rev-parse failed");
    }
    let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(PathBuf::from(root))
}

pub fn origin_url(repo_root: &Path) -> Result<String> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(["config", "--get", "remote.origin.url"])
        .output()
        .context("failed to run git config")?;
    if !output.status.success() {
        bail!("remote.origin.url not found");
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn parse_github_repo(remote: &str) -> Result<(String, String)> {
    let re = Regex::new(r"github\.com[:/]+([^/]+)/([^/]+?)(?:\.git)?$")?;
    if let Some(caps) = re.captures(remote) {
        let owner = caps.get(1).unwrap().as_str().to_string();
        let repo = caps.get(2).unwrap().as_str().to_string();
        return Ok((owner, repo));
    }
    bail!("failed to parse github repo from remote: {}", remote)
}

pub fn create_branch(repo_root: &Path, base: &str, branch: &str) -> Result<()> {
    run_git(repo_root, ["fetch", "origin", base])?;
    run_git(
        repo_root,
        ["checkout", "-B", branch, &format!("origin/{}", base)],
    )?;
    Ok(())
}

pub fn apply_patch(repo_root: &Path, patch: &str) -> Result<()> {
    if try_git_apply(repo_root, patch)? {
        return Ok(());
    }
    apply_loose_unified_patch(repo_root, patch)?;
    Ok(())
}

fn try_git_apply(repo_root: &Path, patch: &str) -> Result<bool> {
    let mut cmd = Command::new("git");
    cmd.current_dir(repo_root)
        .arg("apply")
        .arg("--whitespace=fix")
        .arg("-")
        .stdin(std::process::Stdio::piped());
    let mut child = cmd.spawn().context("failed to spawn git apply")?;
    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        stdin
            .write_all(patch.as_bytes())
            .context("failed to write patch to git apply")?;
    }
    let status = child.wait().context("failed to wait for git apply")?;
    Ok(status.success())
}

fn apply_loose_unified_patch(repo_root: &Path, patch: &str) -> Result<()> {
    let files = parse_loose_patch_files(patch)?;
    if files.is_empty() {
        bail!("patch parsing failed: no files found");
    }

    for file in files {
        let path = resolve_patch_path(repo_root, &file.path)?;
        let original = fs::read_to_string(&path)
            .with_context(|| format!("failed to read patch target: {}", path.display()))?;
        let had_trailing_newline = original.ends_with('\n');
        let mut lines: Vec<String> = original
            .split('\n')
            .map(std::string::ToString::to_string)
            .collect();
        if had_trailing_newline {
            lines.pop();
        }

        for hunk in file.hunks {
            let old_lines: Vec<String> = hunk
                .iter()
                .filter(|line| line.starts_with(' ') || line.starts_with('-'))
                .map(|line| line[1..].to_string())
                .collect();
            let new_lines: Vec<String> = hunk
                .iter()
                .filter(|line| line.starts_with(' ') || line.starts_with('+'))
                .map(|line| line[1..].to_string())
                .collect();

            let start = find_subsequence(&lines, &old_lines)
                .with_context(|| format!("failed to match hunk in {}", path.display()))?;
            let end = start + old_lines.len();
            lines.splice(start..end, new_lines);
        }

        let mut updated = lines.join("\n");
        if had_trailing_newline {
            updated.push('\n');
        }
        fs::write(&path, updated)
            .with_context(|| format!("failed to write patch target: {}", path.display()))?;
    }

    Ok(())
}

#[derive(Debug)]
struct LoosePatchFile {
    path: String,
    hunks: Vec<Vec<String>>,
}

fn parse_loose_patch_files(patch: &str) -> Result<Vec<LoosePatchFile>> {
    let lines: Vec<&str> = patch.lines().collect();
    let mut files = Vec::new();
    let mut i = 0usize;

    while i < lines.len() {
        if !lines[i].starts_with("diff --git ") {
            i += 1;
            continue;
        }

        let parts: Vec<&str> = lines[i].split_whitespace().collect();
        if parts.len() < 4 {
            bail!("invalid diff header: {}", lines[i]);
        }
        let path = parts[3].to_string();
        i += 1;

        let mut hunks: Vec<Vec<String>> = Vec::new();
        while i < lines.len() && !lines[i].starts_with("diff --git ") {
            if lines[i].starts_with("@@") {
                i += 1;
                let mut hunk = Vec::new();
                while i < lines.len()
                    && !lines[i].starts_with("@@")
                    && !lines[i].starts_with("diff --git ")
                {
                    let line = lines[i];
                    if line.starts_with(' ') || line.starts_with('+') || line.starts_with('-') {
                        hunk.push(line.to_string());
                    }
                    i += 1;
                }
                if !hunk.is_empty() {
                    hunks.push(hunk);
                }
                continue;
            }
            i += 1;
        }

        files.push(LoosePatchFile { path, hunks });
    }

    Ok(files)
}

fn resolve_patch_path(repo_root: &Path, raw_path: &str) -> Result<PathBuf> {
    let trimmed = raw_path
        .strip_prefix("b/")
        .or_else(|| raw_path.strip_prefix("a/"))
        .unwrap_or(raw_path);
    let rel = Path::new(trimmed);
    if rel.is_absolute() {
        bail!("absolute patch paths are not allowed: {}", raw_path);
    }
    for component in rel.components() {
        match component {
            Component::Normal(_) => {}
            Component::CurDir => {}
            _ => bail!("unsafe patch path: {}", raw_path),
        }
    }
    Ok(repo_root.join(rel))
}

fn find_subsequence(haystack: &[String], needle: &[String]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

pub fn has_changes(repo_root: &Path) -> Result<bool> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(["status", "--porcelain"])
        .output()
        .context("failed to run git status")?;
    if !output.status.success() {
        bail!("git status failed");
    }
    Ok(!String::from_utf8_lossy(&output.stdout).trim().is_empty())
}

pub fn commit_all(repo_root: &Path, message: &str) -> Result<()> {
    run_git(repo_root, ["add", "-A"])?;
    run_git(repo_root, ["commit", "-m", message])?;
    Ok(())
}

pub fn push(repo_root: &Path, branch: &str) -> Result<()> {
    run_git(repo_root, ["push", "-u", "origin", branch])?;
    Ok(())
}

fn run_git<I, S>(repo_root: &Path, args: I) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let status = Command::new("git")
        .current_dir(repo_root)
        .args(args)
        .status()
        .context("failed to run git command")?;
    if !status.success() {
        bail!("git command failed");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn when_parsing_ssh_remote_should_extract_owner_repo() {
        let parsed = parse_github_repo("git@github.com:acme/project.git").expect("parse");
        assert_eq!(parsed, ("acme".to_string(), "project".to_string()));
    }

    #[test]
    fn when_applying_loose_patch_should_replace_context_block() {
        let temp = tempfile::tempdir().expect("tempdir");
        let file = temp.path().join("README.md");
        fs::write(
            &file,
            "line1\nmacOS (Homebrew):\n```bash\nbrew install skill\n```\nline2\n",
        )
        .expect("write");

        let patch = r#"diff --git a/README.md b/README.md
--- a/README.md
+++ b/README.md
@@
 macOS (Homebrew):
 ```bash
 brew install skill
+skill --version
 ```
"#;

        apply_loose_unified_patch(temp.path(), patch).expect("apply patch");
        let updated = fs::read_to_string(&file).expect("read");
        assert!(updated.contains("brew install skill\nskill --version\n```"));
    }
}
