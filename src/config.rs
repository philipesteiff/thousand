use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowConfig {
    pub version: u32,
    pub name: String,
    pub query: QueryConfig,
    pub agents: AgentsConfig,
    pub outputs: OutputsConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QueryConfig {
    pub prompt: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentsConfig {
    pub scan: AgentConfig,
    pub change: AgentConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentConfig {
    pub model: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OutputsConfig {
    pub github: GithubOutputs,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GithubOutputs {
    pub repo: Option<String>,
    pub issue: Option<IssueConfig>,
    pub pr: Option<PrConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IssueConfig {
    pub labels: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PrConfig {
    pub draft: Option<bool>,
}

impl WorkflowConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read config: {}", path.display()))?;
        let cfg: WorkflowConfig = serde_yaml::from_str(&raw)
            .with_context(|| format!("invalid yaml: {}", path.display()))?;
        Ok(cfg)
    }

    pub fn validate(&self) -> Result<()> {
        if self.version != 1 {
            bail!("unsupported config version: {}", self.version);
        }
        if self.name.trim().is_empty() {
            bail!("config name is required");
        }
        if self.query.prompt.trim().is_empty() {
            bail!("query.prompt is required");
        }
        if self.agents.scan.model.trim().is_empty() {
            bail!("agents.scan.model is required");
        }
        if self.agents.change.model.trim().is_empty() {
            bail!("agents.change.model is required");
        }
        Ok(())
    }

    pub fn github_repo(&self) -> Option<&str> {
        self.outputs.github.repo.as_deref()
    }

    pub fn issue_labels_with_required(&self) -> Vec<String> {
        let mut labels = vec!["thousand".to_string()];
        if let Some(custom) = self
            .outputs
            .github
            .issue
            .as_ref()
            .and_then(|issue| issue.labels.as_ref())
        {
            for label in custom {
                let trimmed = label.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if !labels
                    .iter()
                    .any(|existing| existing.eq_ignore_ascii_case(trimmed))
                {
                    labels.push(trimmed.to_string());
                }
            }
        }
        labels
    }

    pub fn pr_draft(&self) -> bool {
        self.outputs
            .github
            .pr
            .as_ref()
            .and_then(|pr| pr.draft)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_config(yaml: &str) -> WorkflowConfig {
        serde_yaml::from_str(yaml).expect("valid config")
    }

    #[test]
    fn when_no_issue_labels_should_include_required_label() {
        let cfg = parse_config(
            r#"
version: 1
name: test
query:
  prompt: audit
agents:
  scan:
    model: gpt-5-codex
  change:
    model: gpt-5-codex
outputs:
  github:
    repo: owner/repo
"#,
        );

        assert_eq!(
            cfg.issue_labels_with_required(),
            vec!["thousand".to_string()]
        );
    }

    #[test]
    fn when_custom_labels_should_deduplicate_required_label() {
        let cfg = parse_config(
            r#"
version: 1
name: test
query:
  prompt: audit
agents:
  scan:
    model: gpt-5-codex
  change:
    model: gpt-5-codex
outputs:
  github:
    repo: owner/repo
    issue:
      labels: ["security", "Thousand", "ops"]
"#,
        );

        assert_eq!(
            cfg.issue_labels_with_required(),
            vec![
                "thousand".to_string(),
                "security".to_string(),
                "ops".to_string()
            ]
        );
    }
}
