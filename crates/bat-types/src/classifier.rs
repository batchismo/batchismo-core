use serde::{Deserialize, Serialize};

/// Request classification based on complexity, domain, and required capabilities.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RequestClassification {
    pub complexity: Complexity,
    pub domain: Domain,
    pub capabilities: Vec<Capability>,
    /// Confidence score for the classification (0.0 - 1.0).
    pub confidence: f32,
}

/// Complexity levels for request classification.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Complexity {
    Simple,
    Moderate,
    Complex,
}

/// Domain categories for request classification.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Domain {
    Code,
    Creative,
    Analytical,
    Conversational,
}

/// Required capabilities for request classification.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Capability {
    ToolUse,
    LongContext,
    Reasoning,
}

impl Complexity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Complexity::Simple => "simple",
            Complexity::Moderate => "moderate",
            Complexity::Complex => "complex",
        }
    }
}

impl Domain {
    pub fn as_str(&self) -> &'static str {
        match self {
            Domain::Code => "code",
            Domain::Creative => "creative",
            Domain::Analytical => "analytical",
            Domain::Conversational => "conversational",
        }
    }
}

impl Capability {
    pub fn as_str(&self) -> &'static str {
        match self {
            Capability::ToolUse => "tool_use",
            Capability::LongContext => "long_context",
            Capability::Reasoning => "reasoning",
        }
    }
}