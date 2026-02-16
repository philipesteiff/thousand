use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FindingType {
    Security,
    Correctness,
    Reliability,
    Performance,
    Maintainability,
    Test,
    Documentation,
    BusinessLogic,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Critical => "critical",
            Severity::High => "high",
            Severity::Medium => "medium",
            Severity::Low => "low",
        }
    }
}

impl FindingType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FindingType::Security => "security",
            FindingType::Correctness => "correctness",
            FindingType::Reliability => "reliability",
            FindingType::Performance => "performance",
            FindingType::Maintainability => "maintainability",
            FindingType::Test => "test",
            FindingType::Documentation => "documentation",
            FindingType::BusinessLogic => "business-logic",
        }
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::fmt::Display for FindingType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub title: String,
    pub severity: Severity,
    #[serde(rename = "type")]
    pub finding_type: FindingType,
    pub body: String,
    pub id: Option<String>,
    pub status: Option<String>,
    pub evidence_files: Option<Vec<String>>,
    pub suggested_tests: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Change {
    pub summary: String,
    pub patch: String,
    pub tests: Option<Vec<String>>,
}
