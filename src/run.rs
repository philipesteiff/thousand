use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use chrono::Utc;
use regex::Regex;

use crate::codex::CodexRunner;
use crate::config::WorkflowConfig;
use crate::git;
use crate::github::{CreatePullRequest, GitHubClient, IssueDetails};
use crate::types::{Change, Finding};

const REQUIRED_ISSUE_LABEL: &str = "thousand";

struct RepoContext {
    config: WorkflowConfig,
    repo_root: PathBuf,
    owner: String,
    repo: String,
    gh: GitHubClient,
}

pub async fn find(config_path: &Path) -> Result<()> {
    let ctx = load_repo_context(config_path).await?;

    eprintln!("Running find agent...");
    let codex = CodexRunner::new();
    let find_prompt = build_find_prompt(&ctx.repo_root, &ctx.config);
    let finding = codex.run_finding(
        &find_prompt,
        &ctx.config.agents.find.model,
        ctx.config.find_reasoning_effort(),
    )?;
    validate_finding(&finding)?;

    eprintln!("Creating GitHub issue...");
    let issue_title = build_issue_title(&finding);
    let issue_body = build_issue_body(&finding);
    let labels = build_issue_labels(&ctx.config, &finding);
    let issue = ctx
        .gh
        .create_issue(&ctx.owner, &ctx.repo, &issue_title, &issue_body, &labels)
        .await?;

    println!("{}", issue.html_url);
    Ok(())
}

pub async fn solve(config_path: &Path, issue_ref: &str) -> Result<()> {
    let ctx = load_repo_context(config_path).await?;
    let issue_number = resolve_issue_ref(issue_ref, &ctx.owner, &ctx.repo)?;

    eprintln!("Fetching GitHub issue...");
    let issue = ctx
        .gh
        .get_issue(&ctx.owner, &ctx.repo, issue_number)
        .await?;
    if !issue_has_label(&issue, REQUIRED_ISSUE_LABEL) {
        bail!(
            "issue #{} is missing required label `{}`",
            issue.number,
            REQUIRED_ISSUE_LABEL
        );
    }

    eprintln!("Running solve agent...");
    let codex = CodexRunner::new();
    let solve_prompt = build_solve_prompt_from_issue(&ctx.repo_root, &issue);
    let change = codex.run_change(
        &solve_prompt,
        &ctx.config.agents.solve.model,
        ctx.config.solve_reasoning_effort(),
    )?;
    let change_summary = choose_change_summary(&change.summary, &issue.title);

    eprintln!("Applying patch...");
    let default_branch = ctx.gh.get_default_branch(&ctx.owner, &ctx.repo).await?;
    let branch = build_branch_name(&ctx.config.name);
    git::create_branch(&ctx.repo_root, &default_branch, &branch)?;
    git::apply_patch(&ctx.repo_root, &change.patch)?;

    if !git::has_changes(&ctx.repo_root)? {
        bail!("no changes after applying patch");
    }

    let commit_message = format!("thousand: {}", commit_subject_summary(&change_summary));
    git::commit_all(&ctx.repo_root, &commit_message)?;
    git::push(&ctx.repo_root, &branch)?;

    eprintln!("Creating pull request...");
    let pr_title_suffix =
        normalize_summary(&issue.title).unwrap_or_else(|| "Automated update".to_string());
    let pr_title = format!("thousand: {}", pr_title_suffix);
    let pr_body = build_pr_body(issue.number, &change_summary, &change);
    let pr_request = CreatePullRequest {
        title: &pr_title,
        body: &pr_body,
        head: &branch,
        base: &default_branch,
        draft: ctx.config.pr_draft(),
    };
    let pr = ctx
        .gh
        .create_pull_request(&ctx.owner, &ctx.repo, &pr_request)
        .await?;

    eprintln!("Commenting issue with PR link...");
    let issue_comment = format!("Opened PR: {}", pr.html_url);
    ctx.gh
        .create_issue_comment(&ctx.owner, &ctx.repo, issue.number, &issue_comment)
        .await?;

    println!("{}", pr.html_url);
    Ok(())
}

async fn load_repo_context(config_path: &Path) -> Result<RepoContext> {
    let config = WorkflowConfig::load(config_path)?;
    config.validate()?;

    let repo_root = git::repo_root()?;
    let repo_string = match config.github_repo() {
        Some(repo) => repo.to_string(),
        None => {
            let origin = git::origin_url(&repo_root)?;
            let (owner, repo) = git::parse_github_repo(&origin)?;
            format!("{}/{}", owner, repo)
        }
    };
    let (owner, repo) = parse_owner_repo(&repo_string)?;
    let gh = GitHubClient::from_env()?;

    Ok(RepoContext {
        config,
        repo_root,
        owner,
        repo,
        gh,
    })
}

fn parse_owner_repo(repo: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = repo.split('/').collect();
    if parts.len() != 2 {
        bail!("invalid repo format, expected owner/repo: {}", repo);
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

fn resolve_issue_ref(issue_ref: &str, owner: &str, repo: &str) -> Result<u64> {
    let trimmed = issue_ref.trim();
    if let Ok(number) = trimmed.parse::<u64>() {
        if number == 0 {
            bail!("issue number must be > 0");
        }
        return Ok(number);
    }

    let re = Regex::new(r"^https?://github\.com/([^/]+)/([^/]+)/issues/([0-9]+)(?:[/?#].*)?$")
        .context("failed to compile issue URL regex")?;
    let captures = re
        .captures(trimmed)
        .with_context(|| format!("invalid issue reference: {}", issue_ref))?;

    let issue_owner = captures
        .get(1)
        .map(|m| m.as_str())
        .context("missing issue owner in URL")?;
    let issue_repo = captures
        .get(2)
        .map(|m| m.as_str())
        .context("missing issue repo in URL")?;
    let issue_number = captures
        .get(3)
        .map(|m| m.as_str())
        .context("missing issue number in URL")?
        .parse::<u64>()
        .context("invalid issue number in URL")?;

    if !issue_owner.eq_ignore_ascii_case(owner) || !issue_repo.eq_ignore_ascii_case(repo) {
        bail!(
            "issue repo mismatch: expected {}/{}, got {}/{}",
            owner,
            repo,
            issue_owner,
            issue_repo
        );
    }

    Ok(issue_number)
}

fn issue_has_label(issue: &IssueDetails, label: &str) -> bool {
    issue
        .labels
        .iter()
        .any(|current| current.name.eq_ignore_ascii_case(label))
}

fn build_find_prompt(repo_root: &Path, config: &WorkflowConfig) -> String {
    format!(
        "You are an autonomous code auditor.\n\nRepo: {}\n\nTask: {}\n\nReturn ONE finding with concise actionable sections.\nRules:\n- body: short problem statement (1-3 paragraphs).\n- impact: concrete impact/risk in 1-3 bullets.\n- recommendation: concrete fix direction in 1-3 bullets.\n- next_step: immediate first implementation step (one short sentence or up to 3 bullets).\n- Avoid long background and repetition.\n\nReturn ONLY valid JSON that matches the provided schema. Do not include markdown fences or extra text.",
        repo_root.display(),
        config.query.prompt
    )
}

fn build_solve_prompt_from_issue(repo_root: &Path, issue: &IssueDetails) -> String {
    let mut prompt = String::new();
    prompt.push_str("You are an autonomous code fixer.\n\n");
    prompt.push_str(&format!("Repo: {}\n", repo_root.display()));
    prompt.push_str(&format!("Issue URL: {}\n", issue.html_url));
    prompt.push_str(&format!("Issue Title: {}\n\n", issue.title));
    prompt.push_str("Issue Body:\n");
    let issue_body = issue.body.as_deref().unwrap_or("").trim();
    if issue_body.is_empty() {
        prompt.push_str("(empty)\n");
    } else {
        prompt.push_str(issue_body);
        prompt.push('\n');
    }
    prompt.push_str("\nProduce a valid git unified diff patch against the repo root.\n");
    prompt.push_str("Patch format requirements:\n");
    prompt.push_str(
        "- Use full git diff sections starting with `diff --git a/<path> b/<path>` for every changed file.\n",
    );
    prompt
        .push_str("- Include matching `--- a/<path>` and `+++ b/<path>` headers for every file.\n");
    prompt.push_str("- Include one or more `@@ ... @@` hunks under each file header.\n");
    prompt.push_str(
        "- Do not include commentary, markdown fences, or partial hunks without file headers.\n",
    );
    prompt.push_str("Return ONLY valid JSON that matches the provided schema. Do not include markdown fences or extra text.\n");
    prompt
}

fn build_issue_title(finding: &Finding) -> String {
    finding.title.clone()
}

fn build_issue_body(finding: &Finding) -> String {
    let mut body = String::new();
    body.push_str("## Finding\n\n");
    let finding_text = finding.body.trim();
    if finding_text.is_empty() {
        body.push_str("No finding details were provided.\n");
    } else {
        body.push_str(finding_text);
        body.push('\n');
    }

    body.push_str("\n## Impact\n\n");
    body.push_str(finding.impact.trim());
    body.push('\n');

    body.push_str("\n## Recommendation\n\n");
    body.push_str(finding.recommendation.trim());
    body.push('\n');

    body.push_str("\n## Next Step\n\n");
    body.push_str(finding.next_step.trim());
    body.push('\n');

    body
}

fn validate_finding(finding: &Finding) -> Result<()> {
    validate_finding_field("title", &finding.title, 120)?;
    validate_finding_field("body", &finding.body, 1200)?;
    validate_finding_field("impact", &finding.impact, 800)?;
    validate_finding_field("recommendation", &finding.recommendation, 800)?;
    validate_finding_field("next_step", &finding.next_step, 400)?;
    Ok(())
}

fn validate_finding_field(name: &str, value: &str, max_len: usize) -> Result<()> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        bail!("finding.{} must be non-empty", name);
    }
    if trimmed.len() > max_len {
        bail!("finding.{} exceeds {} characters", name, max_len);
    }
    Ok(())
}

fn build_issue_labels(config: &WorkflowConfig, finding: &Finding) -> Vec<String> {
    let mut labels = config.issue_labels_with_required();
    upsert_label(&mut labels, format!("severity:{}", finding.severity));
    upsert_label(&mut labels, format!("type:{}", finding.finding_type));
    labels
}

fn upsert_label(labels: &mut Vec<String>, label: String) {
    labels.retain(|existing| !existing.eq_ignore_ascii_case(&label));
    labels.push(label);
}

fn build_pr_body(issue_number: u64, change_summary: &str, change: &Change) -> String {
    let mut body = String::new();
    body.push_str(&format!("Resolves: #{}\n\n", issue_number));
    body.push_str("## Summary\n");
    body.push_str(change_summary);
    body.push('\n');
    if let Some(tests) = &change.tests {
        if !tests.is_empty() {
            body.push_str("\n## Tests\n");
            for test in tests {
                body.push_str(&format!("- {}\n", test));
            }
        }
    }
    body
}

fn choose_change_summary(change_summary_raw: &str, fallback_summary_raw: &str) -> String {
    if let Some(summary) = normalize_summary(change_summary_raw) {
        return summary;
    }
    if let Some(fallback) = normalize_summary(fallback_summary_raw) {
        return fallback;
    }
    "Apply automated update".to_string()
}

fn normalize_summary(raw: &str) -> Option<String> {
    let normalized = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        return None;
    }
    if !normalized.chars().any(|c| c.is_alphanumeric()) {
        return None;
    }
    if normalized.starts_with('{') || normalized.starts_with('[') {
        return None;
    }
    Some(normalized)
}

fn commit_subject_summary(summary: &str) -> String {
    const MAX_LEN: usize = 72;
    if summary.len() <= MAX_LEN {
        return summary.to_string();
    }

    let mut truncated = summary.chars().take(MAX_LEN - 3).collect::<String>();
    if let Some(idx) = truncated.rfind(' ') {
        // Keep commit subjects readable when truncating long model output.
        truncated.truncate(idx);
    }
    truncated.push_str("...");
    truncated
}

fn build_branch_name(workflow_name: &str) -> String {
    let slug = workflow_name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    let ts = Utc::now().format("%Y%m%d%H%M%S");
    format!(
        "thousand/{}/{}",
        if slug.is_empty() { "workflow" } else { &slug },
        ts
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::IssueLabel;
    use crate::types::{Change, FindingType, Severity};

    fn sample_finding() -> Finding {
        Finding {
            title: "Improve README installation guidance".to_string(),
            severity: Severity::Low,
            finding_type: FindingType::Documentation,
            body: "Add a verification step after installation.".to_string(),
            impact: "Users may have setup confusion after install.".to_string(),
            recommendation: "Document a quick verification command after installation.".to_string(),
            next_step: "Add one verification snippet under each install method in README."
                .to_string(),
        }
    }

    fn sample_issue() -> IssueDetails {
        IssueDetails {
            number: 5,
            title: "Fix docs".to_string(),
            body: Some("Issue details".to_string()),
            html_url: "https://github.com/acme/project/issues/5".to_string(),
            labels: vec![IssueLabel {
                name: "thousand".to_string(),
            }],
        }
    }

    fn sample_workflow_config(yaml: &str) -> WorkflowConfig {
        serde_yaml::from_str(yaml).expect("config")
    }

    #[test]
    fn when_change_summary_is_invalid_should_fallback_to_issue_title() {
        let summary = choose_change_summary("{", "Improve README installation guidance");
        assert_eq!(summary, "Improve README installation guidance");
    }

    #[test]
    fn when_building_pr_body_should_include_structured_sections() {
        let change = Change {
            summary: "Add install verification snippet".to_string(),
            patch: "diff --git a/README.md b/README.md".to_string(),
            tests: Some(vec!["cargo test -p skill-cli --test help".to_string()]),
        };

        let body = build_pr_body(5, "Add install verification snippet", &change);
        assert!(body.contains("Resolves: #5"));
        assert!(body.contains("## Summary"));
        assert!(body.contains("Add install verification snippet"));
        assert!(body.contains("## Tests"));
    }

    #[test]
    fn when_building_issue_body_should_include_structured_sections() {
        let finding = sample_finding();
        let body = build_issue_body(&finding);

        assert!(body.contains("## Finding"));
        assert!(body.contains("## Impact"));
        assert!(body.contains("## Recommendation"));
        assert!(body.contains("## Next Step"));
        assert!(!body.contains("## Metadata"));
    }

    #[test]
    fn when_building_issue_labels_should_include_severity_and_type() {
        let cfg = sample_workflow_config(
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
        let labels = build_issue_labels(&cfg, &sample_finding());
        assert!(labels.iter().any(|label| label == "thousand"));
        assert!(labels.iter().any(|label| label == "severity:low"));
        assert!(labels.iter().any(|label| label == "type:documentation"));
    }

    #[test]
    fn when_building_issue_labels_should_dedupe_case_insensitive() {
        let cfg = sample_workflow_config(
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
      labels: ["Severity:Low", "Type:documentation", "ops"]
"#,
        );
        let labels = build_issue_labels(&cfg, &sample_finding());
        assert!(labels.iter().any(|label| label == "thousand"));
        assert!(labels.iter().any(|label| label == "severity:low"));
        assert!(labels.iter().any(|label| label == "type:documentation"));
        assert!(labels.iter().any(|label| label == "ops"));
        assert_eq!(
            labels
                .iter()
                .filter(|label| label.eq_ignore_ascii_case("severity:low"))
                .count(),
            1
        );
        assert_eq!(
            labels
                .iter()
                .filter(|label| label.eq_ignore_ascii_case("type:documentation"))
                .count(),
            1
        );
    }

    #[test]
    fn when_finding_required_field_is_empty_should_fail_validation() {
        let mut finding = sample_finding();
        finding.impact = "  ".to_string();
        let err = validate_finding(&finding).expect_err("expected validation error");
        assert!(err.to_string().contains("finding.impact must be non-empty"));
    }

    #[test]
    fn when_finding_field_is_too_long_should_fail_validation() {
        let mut finding = sample_finding();
        finding.next_step = "a".repeat(401);
        let err = validate_finding(&finding).expect_err("expected validation error");
        assert!(err
            .to_string()
            .contains("finding.next_step exceeds 400 characters"));
    }

    #[test]
    fn when_commit_summary_is_long_should_truncate_for_subject() {
        let text = "This summary is intentionally very long so we can verify commit subjects are trimmed without clipping every line in the pull request body.";
        let subject = commit_subject_summary(text);
        assert!(subject.len() <= 72);
        assert!(subject.ends_with("..."));
    }

    #[test]
    fn when_issue_ref_is_number_should_parse() {
        let parsed = resolve_issue_ref("42", "acme", "project").expect("parse");
        assert_eq!(parsed, 42);
    }

    #[test]
    fn when_issue_ref_is_url_for_same_repo_should_parse() {
        let parsed = resolve_issue_ref(
            "https://github.com/acme/project/issues/42",
            "acme",
            "project",
        )
        .expect("parse");
        assert_eq!(parsed, 42);
    }

    #[test]
    fn when_issue_ref_url_is_different_repo_should_fail() {
        let err = resolve_issue_ref(
            "https://github.com/other/project/issues/42",
            "acme",
            "project",
        )
        .expect_err("expected mismatch");
        assert!(err.to_string().contains("issue repo mismatch"));
    }

    #[test]
    fn when_issue_ref_is_invalid_should_fail() {
        let err = resolve_issue_ref("abc", "acme", "project").expect_err("invalid");
        assert!(err.to_string().contains("invalid issue reference"));
    }

    #[test]
    fn when_issue_has_required_label_should_match_case_insensitive() {
        let mut issue = sample_issue();
        issue.labels = vec![IssueLabel {
            name: "ThOuSaNd".to_string(),
        }];
        assert!(issue_has_label(&issue, "thousand"));
    }
}
