use anyhow::{bail, Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct GitHubClient {
    client: reqwest::Client,
}

impl GitHubClient {
    pub fn from_env() -> Result<Self> {
        let token = std::env::var("THOUSAND_GH_TOKEN")
            .or_else(|_| std::env::var("GITHUB_TOKEN"))
            .context("missing THOUSAND_GH_TOKEN or GITHUB_TOKEN")?;

        let mut headers = HeaderMap::new();
        let auth_value = format!("Bearer {}", token);
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&auth_value).context("invalid token")?,
        );
        headers.insert(USER_AGENT, HeaderValue::from_static("thousand-cli"));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .context("failed to build http client")?;

        Ok(Self { client })
    }

    pub async fn get_default_branch(&self, owner: &str, repo: &str) -> Result<String> {
        let url = format!("https://api.github.com/repos/{}/{}", owner, repo);
        let resp = self.client.get(url).send().await?;
        if !resp.status().is_success() {
            bail!("failed to fetch repo info: {}", resp.status());
        }
        let repo: RepoInfo = resp.json().await?;
        Ok(repo.default_branch)
    }

    pub async fn get_issue(&self, owner: &str, repo: &str, number: u64) -> Result<IssueDetails> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/issues/{}",
            owner, repo, number
        );
        let resp = self.client.get(url).send().await?;
        if !resp.status().is_success() {
            bail!("failed to fetch issue #{}: {}", number, resp.status());
        }
        Ok(resp.json::<IssueDetails>().await?)
    }

    pub async fn create_issue(
        &self,
        owner: &str,
        repo: &str,
        title: &str,
        body: &str,
        labels: &[String],
    ) -> Result<Issue> {
        let url = format!("https://api.github.com/repos/{}/{}/issues", owner, repo);
        let payload = IssueRequest {
            title,
            body,
            labels,
        };
        let resp = self.client.post(url).json(&payload).send().await?;
        if !resp.status().is_success() {
            bail!("failed to create issue: {}", resp.status());
        }
        Ok(resp.json::<Issue>().await?)
    }

    pub async fn create_issue_comment(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
        body: &str,
    ) -> Result<()> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/issues/{}/comments",
            owner, repo, number
        );
        let payload = IssueCommentRequest { body };
        let resp = self.client.post(url).json(&payload).send().await?;
        if !resp.status().is_success() {
            bail!(
                "failed to create issue comment for #{}: {}",
                number,
                resp.status()
            );
        }
        Ok(())
    }

    pub async fn create_pull_request(
        &self,
        owner: &str,
        repo: &str,
        input: &CreatePullRequest<'_>,
    ) -> Result<PullRequest> {
        let url = format!("https://api.github.com/repos/{}/{}/pulls", owner, repo);
        let payload = PullRequestRequest {
            title: input.title,
            body: input.body,
            head: input.head,
            base: input.base,
            draft: input.draft,
        };
        let resp = self.client.post(url).json(&payload).send().await?;
        if !resp.status().is_success() {
            bail!("failed to create pull request: {}", resp.status());
        }
        Ok(resp.json::<PullRequest>().await?)
    }
}

pub struct CreatePullRequest<'a> {
    pub title: &'a str,
    pub body: &'a str,
    pub head: &'a str,
    pub base: &'a str,
    pub draft: bool,
}

#[derive(Debug, Deserialize)]
struct RepoInfo {
    default_branch: String,
}

#[derive(Debug, Serialize)]
struct IssueRequest<'a> {
    title: &'a str,
    body: &'a str,
    labels: &'a [String],
}

#[derive(Debug, Serialize)]
struct IssueCommentRequest<'a> {
    body: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct Issue {
    pub number: u64,
    pub html_url: String,
}

#[derive(Debug, Deserialize)]
pub struct IssueDetails {
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub html_url: String,
    pub labels: Vec<IssueLabel>,
}

#[derive(Debug, Deserialize)]
pub struct IssueLabel {
    pub name: String,
}

#[derive(Debug, Serialize)]
struct PullRequestRequest<'a> {
    title: &'a str,
    body: &'a str,
    head: &'a str,
    base: &'a str,
    draft: bool,
}

#[derive(Debug, Deserialize)]
pub struct PullRequest {
    pub number: u64,
    pub html_url: String,
}
