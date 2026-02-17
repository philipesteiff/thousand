use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};
use tempfile::NamedTempFile;

use crate::types::{Change, Finding};

const FINDING_SCHEMA: &str = r#"{
  "type": "object",
  "required": ["title", "severity", "type", "body", "impact", "recommendation", "next_step"],
  "properties": {
    "title": {"type": "string"},
    "severity": {"type": "string", "enum": ["critical", "high", "medium", "low"]},
    "type": {
      "type": "string",
      "enum": [
        "security",
        "correctness",
        "reliability",
        "performance",
        "maintainability",
        "test",
        "documentation",
        "business-logic"
      ]
    },
    "body": {"type": "string"},
    "impact": {"type": "string"},
    "recommendation": {"type": "string"},
    "next_step": {"type": "string"}
  },
  "additionalProperties": false
}
"#;

const CHANGE_SCHEMA: &str = r#"{
  "type": "object",
  "required": ["summary", "patch", "tests"],
  "properties": {
    "summary": {"type": "string"},
    "patch": {"type": "string"},
    "tests": {"type": ["array", "null"], "items": {"type": "string"}}
  },
  "additionalProperties": false
}
"#;

pub struct CodexRunner {
    binary: String,
}

impl Default for CodexRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl CodexRunner {
    pub fn new() -> Self {
        Self {
            binary: "codex".to_string(),
        }
    }

    pub fn run_finding(
        &self,
        prompt: &str,
        model: &str,
        reasoning_effort: &str,
    ) -> Result<Finding> {
        let output = self.exec(prompt, FINDING_SCHEMA, model, reasoning_effort)?;
        let finding: Finding =
            serde_json::from_str(output.trim()).context("failed to parse finding json")?;
        Ok(finding)
    }

    pub fn run_change(&self, prompt: &str, model: &str, reasoning_effort: &str) -> Result<Change> {
        let output = self.exec(prompt, CHANGE_SCHEMA, model, reasoning_effort)?;
        let change: Change =
            serde_json::from_str(output.trim()).context("failed to parse change json")?;
        Ok(change)
    }

    fn exec(
        &self,
        prompt: &str,
        schema: &str,
        model: &str,
        reasoning_effort: &str,
    ) -> Result<String> {
        let mut schema_file = NamedTempFile::new().context("failed to create schema temp file")?;
        schema_file
            .write_all(schema.as_bytes())
            .context("failed to write schema file")?;
        schema_file.flush().context("failed to flush schema file")?;

        let output_file = NamedTempFile::new().context("failed to create output temp file")?;

        let mut cmd = Command::new(&self.binary);
        cmd.arg("exec")
            .arg("--output-schema")
            .arg(schema_file.path())
            .arg("--output-last-message")
            .arg(output_file.path())
            .arg("-c")
            .arg(reasoning_effort_override(reasoning_effort))
            .arg("--sandbox")
            .arg("read-only")
            .arg("--model")
            .arg(model)
            .stdin(Stdio::piped());

        let mut child = cmd.spawn().context("failed to spawn codex")?;
        if let Some(stdin) = child.stdin.as_mut() {
            stdin
                .write_all(prompt.as_bytes())
                .context("failed to write prompt to codex stdin")?;
        }
        let status = child.wait().context("failed to wait for codex")?;
        if !status.success() {
            bail!("codex exec failed with status: {}", status);
        }

        let output =
            fs::read_to_string(output_file.path()).context("failed to read codex output")?;
        Ok(output)
    }
}

fn reasoning_effort_override(raw: &str) -> String {
    let escaped = raw.replace('\\', "\\\\").replace('"', "\\\"");
    format!("model_reasoning_effort=\"{}\"", escaped)
}

#[cfg(test)]
mod tests {
    use super::reasoning_effort_override;

    #[test]
    fn when_reasoning_effort_contains_quotes_should_escape_for_codex_config_arg() {
        let arg = reasoning_effort_override("high\"custom");
        assert_eq!(arg, "model_reasoning_effort=\"high\\\"custom\"");
    }
}
