use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_yaml::Value;

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
    pub find: AgentConfig,
    pub solve: AgentConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentConfig {
    pub model: String,
    pub reasoning_effort: Option<String>,
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
        Self::parse(&raw).with_context(|| format!("failed to parse config: {}", path.display()))
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
        if self.agents.find.model.trim().is_empty() {
            bail!("agents.find.model is required");
        }
        if self.agents.solve.model.trim().is_empty() {
            bail!("agents.solve.model is required");
        }
        if self
            .agents
            .find
            .reasoning_effort
            .as_deref()
            .is_some_and(|effort| effort.trim().is_empty())
        {
            bail!("agents.find.reasoning_effort must be non-empty when provided");
        }
        if self
            .agents
            .solve
            .reasoning_effort
            .as_deref()
            .is_some_and(|effort| effort.trim().is_empty())
        {
            bail!("agents.solve.reasoning_effort must be non-empty when provided");
        }
        Ok(())
    }

    fn parse(raw: &str) -> Result<Self> {
        let doc: Value = serde_yaml::from_str(raw).context("invalid yaml")?;
        Self::validate_legacy_agent_keys(&doc)?;
        let cfg: WorkflowConfig = serde_yaml::from_value(doc).context("invalid config schema")?;
        Ok(cfg)
    }

    fn validate_legacy_agent_keys(doc: &Value) -> Result<()> {
        let Some(agents) = doc.get("agents").and_then(Value::as_mapping) else {
            return Ok(());
        };

        let has_scan = agents.contains_key(Value::String("scan".to_string()));
        let has_change = agents.contains_key(Value::String("change".to_string()));
        if !has_scan && !has_change {
            return Ok(());
        }

        let mut migrations = Vec::new();
        if has_scan {
            migrations.push("agents.scan -> agents.find");
        }
        if has_change {
            migrations.push("agents.change -> agents.solve");
        }
        bail!(
            "deprecated agent keys in workflow config: {}. Use agents.find and agents.solve.",
            migrations.join(", ")
        );
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

    pub fn find_reasoning_effort(&self) -> &str {
        self.agents
            .find
            .reasoning_effort
            .as_deref()
            .map(str::trim)
            .filter(|effort| !effort.is_empty())
            .unwrap_or("low")
    }

    pub fn solve_reasoning_effort(&self) -> &str {
        self.agents
            .solve
            .reasoning_effort
            .as_deref()
            .map(str::trim)
            .filter(|effort| !effort.is_empty())
            .unwrap_or("low")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_config(yaml: &str) -> WorkflowConfig {
        WorkflowConfig::parse(yaml).expect("valid config")
    }

    fn valid_yaml(reasoning: &str) -> String {
        format!(
            r#"
version: 1
name: test
query:
  prompt: audit
agents:
  find:
    model: gpt-5-codex
    reasoning_effort: {reasoning}
  solve:
    model: gpt-5-codex
outputs:
  github:
    repo: owner/repo
"#
        )
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
  find:
    model: gpt-5-codex
  solve:
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
  find:
    model: gpt-5-codex
  solve:
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

    #[test]
    fn when_reasoning_effort_missing_should_default_to_low() {
        let cfg = parse_config(
            r#"
version: 1
name: test
query:
  prompt: audit
agents:
  find:
    model: gpt-5-codex
  solve:
    model: gpt-5-codex
outputs:
  github:
    repo: owner/repo
"#,
        );

        assert_eq!(cfg.find_reasoning_effort(), "low");
        assert_eq!(cfg.solve_reasoning_effort(), "low");
    }

    #[test]
    fn when_reasoning_effort_blank_should_fail_validation() {
        let cfg = parse_config(&valid_yaml("\"  \""));
        let err = cfg.validate().expect_err("expected validation error");
        assert!(err
            .to_string()
            .contains("agents.find.reasoning_effort must be non-empty"));
    }

    #[test]
    fn when_legacy_agent_keys_used_should_fail_parse_with_migration_hint() {
        let err = WorkflowConfig::parse(
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
        )
        .expect_err("expected parse error");

        let text = err.to_string();
        assert!(text.contains("agents.scan -> agents.find"));
        assert!(text.contains("agents.change -> agents.solve"));
    }
}
