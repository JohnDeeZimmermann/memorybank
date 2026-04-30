use std::fmt;
use std::path::PathBuf;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum DocumentType {
    Commit,
    Plan,
    Research,
}

impl DocumentType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Commit => "COMMIT",
            Self::Plan => "PLAN",
            Self::Research => "RESEARCH",
        }
    }

    pub fn from_db(value: &str) -> Option<Self> {
        match value {
            "COMMIT" => Some(Self::Commit),
            "PLAN" => Some(Self::Plan),
            "RESEARCH" => Some(Self::Research),
            _ => None,
        }
    }
}

impl fmt::Display for DocumentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for DocumentType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DocumentTypeVisitor;

        impl Visitor<'_> for DocumentTypeVisitor {
            type Value = DocumentType;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("one of COMMIT, PLAN, or RESEARCH")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                DocumentType::from_db(value)
                    .ok_or_else(|| E::custom("type must be one of COMMIT, PLAN, or RESEARCH"))
            }
        }

        deserializer.deserialize_str(DocumentTypeVisitor)
    }
}

#[derive(Debug, Clone)]
pub struct Document {
    pub id: String,
    pub document_path: PathBuf,
    pub created_at: String,
    pub invalidated: bool,
    pub invalidation_reason: Option<String>,
    pub quick_summary: String,
    pub document_type: DocumentType,
}

#[derive(Debug, Clone)]
pub struct DocumentSummary {
    pub id: String,
    pub created_at: String,
    pub invalidated: bool,
    pub invalidation_reason: Option<String>,
    pub quick_summary: String,
    pub document_type: DocumentType,
    pub related_files: Vec<String>,
}

impl From<Document> for DocumentSummary {
    fn from(document: Document) -> Self {
        Self {
            id: document.id,
            created_at: document.created_at,
            invalidated: document.invalidated,
            invalidation_reason: document.invalidation_reason,
            quick_summary: document.quick_summary,
            document_type: document.document_type,
            related_files: Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AddDocumentInput {
    pub document: String,
    pub summary: String,
    #[serde(default)]
    pub related_files: Vec<PathBuf>,
    #[serde(default)]
    pub related_documents: Vec<String>,
    #[serde(rename = "type")]
    pub document_type: DocumentType,
}
