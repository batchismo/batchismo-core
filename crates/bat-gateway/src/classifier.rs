use bat_types::classifier::{RequestClassification, Complexity, Domain, Capability};

/// Rule-based request classifier.
pub struct RequestClassifier;

impl RequestClassifier {
    /// Classify a request based on its content and context.
    pub fn classify(content: &str, images: &[bat_types::message::ImageAttachment]) -> RequestClassification {
        let text = content.trim();
        let word_count = text.split_whitespace().count();
        let char_count = text.len();
        
        // Classify complexity based on message length and content patterns
        let complexity = Self::classify_complexity(text, word_count, char_count);
        
        // Classify domain based on keywords and content patterns
        let domain = Self::classify_domain(text);
        
        // Determine required capabilities
        let capabilities = Self::classify_capabilities(text, images, word_count);
        
        // Calculate confidence based on keyword matches and other signals
        let confidence = Self::calculate_confidence(text, &complexity, &domain, &capabilities);
        
        RequestClassification {
            complexity,
            domain,
            capabilities,
            confidence,
        }
    }
    
    fn classify_complexity(text: &str, word_count: usize, char_count: usize) -> Complexity {
        let text_lower = text.to_lowercase();
        
        // Complex indicators
        let complex_keywords = [
            "analyze", "implement", "design", "architect", "refactor", "optimize", 
            "debug", "algorithm", "database", "performance", "security", "infrastructure",
            "comparison", "evaluation", "research", "investigation", "comprehensive",
            "detailed", "in-depth", "thorough", "step-by-step", "systematic"
        ];
        
        let complex_patterns = [
            "how to", "explain how", "walk me through", "break down", "compare",
            "pros and cons", "advantages and disadvantages", "best practices"
        ];
        
        // Check for complex indicators
        let has_complex_keywords = complex_keywords.iter()
            .any(|keyword| text_lower.contains(keyword));
        
        let has_complex_patterns = complex_patterns.iter()
            .any(|pattern| text_lower.contains(pattern));
        
        // Check for code blocks
        let has_code_blocks = text.contains("```") || text.contains("`");
        
        // Check for multi-step instructions
        let has_numbered_steps = text.matches(|c: char| c.is_ascii_digit()).count() > 3
            && (text.contains("1.") || text.contains("step"));
        
        // Simple indicators
        let simple_keywords = [
            "hi", "hello", "thanks", "thank you", "yes", "no", "ok", "okay",
            "what is", "who is", "when", "where", "why", "how much"
        ];
        
        let has_simple_keywords = simple_keywords.iter()
            .any(|keyword| text_lower.contains(keyword));
        
        // Question patterns that are usually simple
        let simple_questions = text.ends_with('?') && word_count <= 10;
        
        // Classify based on indicators
        if has_complex_keywords || has_complex_patterns || has_code_blocks || has_numbered_steps {
            Complexity::Complex
        } else if char_count > 500 || word_count > 100 || (!simple_questions && !has_simple_keywords) {
            Complexity::Moderate
        } else {
            Complexity::Simple
        }
    }
    
    fn classify_domain(text: &str) -> Domain {
        let text_lower = text.to_lowercase();
        
        // Code-related keywords
        let code_keywords = [
            "code", "programming", "function", "variable", "class", "method",
            "algorithm", "debug", "compile", "syntax", "api", "database",
            "git", "commit", "pull request", "repository", "branch",
            "javascript", "python", "rust", "go", "java", "c++", "sql",
            "react", "nodejs", "typescript", "html", "css", "json",
            "error", "exception", "bug", "fix", "refactor", "implement"
        ];
        
        // Creative keywords
        let creative_keywords = [
            "story", "write", "creative", "poem", "narrative", "character",
            "plot", "dialogue", "essay", "article", "blog", "content",
            "brainstorm", "ideas", "imagine", "create", "design", "art",
            "marketing", "copy", "advertisement", "slogan", "headline"
        ];
        
        // Analytical keywords  
        let analytical_keywords = [
            "analyze", "analysis", "data", "statistics", "research", "study",
            "report", "summary", "comparison", "evaluate", "assessment",
            "metrics", "performance", "optimization", "efficiency", "results",
            "trends", "patterns", "correlation", "hypothesis", "conclusion",
            "business", "strategy", "market", "financial", "revenue", "growth"
        ];
        
        // Conversational keywords (catch-all for general chat)
        let conversational_keywords = [
            "hi", "hello", "how are", "what's up", "thanks", "please",
            "opinion", "think", "feel", "believe", "personal", "experience",
            "tell me", "explain", "help", "question", "advice", "suggest"
        ];
        
        // Count keyword matches for each domain
        let code_matches = code_keywords.iter()
            .filter(|&keyword| text_lower.contains(keyword))
            .count();
            
        let creative_matches = creative_keywords.iter()
            .filter(|&keyword| text_lower.contains(keyword))
            .count();
            
        let analytical_matches = analytical_keywords.iter()
            .filter(|&keyword| text_lower.contains(keyword))
            .count();
            
        let conversational_matches = conversational_keywords.iter()
            .filter(|&keyword| text_lower.contains(keyword))
            .count();
        
        // Check for code patterns
        let has_code_blocks = text.contains("```") || text.contains("`");
        let has_file_extensions = text.contains(".js") || text.contains(".py") 
            || text.contains(".rs") || text.contains(".json") || text.contains(".html");
        
        // Determine domain based on highest match count and patterns
        if has_code_blocks || has_file_extensions || code_matches > 0 {
            Domain::Code
        } else if creative_matches > analytical_matches && creative_matches > conversational_matches {
            Domain::Creative
        } else if analytical_matches > conversational_matches {
            Domain::Analytical
        } else {
            Domain::Conversational
        }
    }
    
    fn classify_capabilities(text: &str, images: &[bat_types::message::ImageAttachment], word_count: usize) -> Vec<Capability> {
        let text_lower = text.to_lowercase();
        let mut capabilities = Vec::new();
        
        // Tool use indicators
        let tool_keywords = [
            "file", "folder", "directory", "read", "write", "create", "delete",
            "run", "execute", "command", "script", "install", "download",
            "open", "save", "search", "find", "list", "copy", "move",
            "git", "commit", "push", "pull", "clone", "branch",
            "terminal", "shell", "bash", "powershell", "cmd"
        ];
        
        let needs_tool_use = tool_keywords.iter()
            .any(|keyword| text_lower.contains(keyword))
            || text.contains("./") || text.contains("../") || text.contains("C:\\")
            || text.contains("/home/") || text.contains("/usr/");
        
        if needs_tool_use {
            capabilities.push(Capability::ToolUse);
        }
        
        // Long context indicators
        let long_context_keywords = [
            "entire", "whole", "all", "complete", "full", "comprehensive",
            "detailed", "thorough", "in-depth", "extensive", "multiple files",
            "codebase", "project", "repository", "documentation"
        ];
        
        let needs_long_context = word_count > 200
            || long_context_keywords.iter().any(|keyword| text_lower.contains(keyword))
            || images.len() > 2; // Multiple images suggest need for long context
        
        if needs_long_context {
            capabilities.push(Capability::LongContext);
        }
        
        // Reasoning indicators
        let reasoning_keywords = [
            "why", "how", "explain", "reason", "because", "analyze", "compare",
            "evaluate", "decide", "choose", "recommend", "suggest", "strategy",
            "approach", "solution", "problem", "issue", "challenge", "optimize",
            "improve", "best", "better", "worse", "pros", "cons", "trade-off",
            "consider", "think", "logic", "rational", "conclusion", "inference"
        ];
        
        let reasoning_patterns = [
            "what if", "how about", "what would", "should i", "which is better",
            "pros and cons", "advantages and disadvantages", "best practices"
        ];
        
        let needs_reasoning = reasoning_keywords.iter()
            .any(|keyword| text_lower.contains(keyword))
            || reasoning_patterns.iter().any(|pattern| text_lower.contains(pattern))
            || text.contains('?'); // Questions often require reasoning
        
        if needs_reasoning {
            capabilities.push(Capability::Reasoning);
        }
        
        capabilities
    }
    
    fn calculate_confidence(text: &str, complexity: &Complexity, domain: &Domain, capabilities: &[Capability]) -> f32 {
        let mut confidence: f32 = 0.5; // Start with neutral confidence
        
        let text_lower = text.to_lowercase();
        let word_count = text.split_whitespace().count();
        
        // Increase confidence for clear indicators
        match complexity {
            Complexity::Simple => {
                if word_count <= 20 && (text.ends_with('?') || text_lower.starts_with("hi") || text_lower.starts_with("hello")) {
                    confidence += 0.3f32;
                }
            }
            Complexity::Complex => {
                if text.contains("```") || word_count > 100 {
                    confidence += 0.3f32;
                }
            }
            _ => {}
        }
        
        // Domain-specific confidence boosts
        match domain {
            Domain::Code => {
                if text.contains("```") || text.contains("function") || text.contains("class") {
                    confidence += 0.2f32;
                }
            }
            Domain::Creative => {
                if text_lower.contains("write") || text_lower.contains("story") {
                    confidence += 0.2f32;
                }
            }
            Domain::Analytical => {
                if text_lower.contains("analyze") || text_lower.contains("data") {
                    confidence += 0.2f32;
                }
            }
            _ => {}
        }
        
        // Capability confidence
        if capabilities.contains(&Capability::ToolUse) && (text.contains("/") || text_lower.contains("file")) {
            confidence += 0.1f32;
        }
        
        // Clamp between 0.1 and 1.0
        confidence.max(0.1f32).min(1.0f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_simple_greeting() {
        let classification = RequestClassifier::classify("Hi there!", &[]);
        assert_eq!(classification.complexity, Complexity::Simple);
        assert_eq!(classification.domain, Domain::Conversational);
        assert!(classification.capabilities.is_empty());
        assert!(classification.confidence > 0.5);
    }
    
    #[test]
    fn test_simple_question() {
        let classification = RequestClassifier::classify("What is the capital of France?", &[]);
        assert_eq!(classification.complexity, Complexity::Simple);
        assert_eq!(classification.domain, Domain::Conversational);
        assert!(classification.capabilities.contains(&Capability::Reasoning));
    }
    
    #[test]
    fn test_code_request() {
        let classification = RequestClassifier::classify(
            "Can you help me implement a function in Rust to parse JSON?",
            &[]
        );
        assert_eq!(classification.complexity, Complexity::Moderate);
        assert_eq!(classification.domain, Domain::Code);
        assert!(classification.capabilities.contains(&Capability::Reasoning));
    }
    
    #[test]
    fn test_complex_code_with_blocks() {
        let classification = RequestClassifier::classify(
            "Debug this Python code:\n```python\ndef func():\n    return x + 1\n```",
            &[]
        );
        assert_eq!(classification.complexity, Complexity::Complex);
        assert_eq!(classification.domain, Domain::Code);
        assert!(classification.capabilities.contains(&Capability::Reasoning));
    }
    
    #[test]
    fn test_file_operation() {
        let classification = RequestClassifier::classify(
            "Read the contents of config.json and create a backup",
            &[]
        );
        assert_eq!(classification.domain, Domain::Code);
        assert!(classification.capabilities.contains(&Capability::ToolUse));
    }
    
    #[test]
    fn test_creative_request() {
        let classification = RequestClassifier::classify(
            "Write a short story about a robot discovering emotions",
            &[]
        );
        assert_eq!(classification.domain, Domain::Creative);
        assert!(classification.capabilities.contains(&Capability::Reasoning));
    }
    
    #[test]
    fn test_analytical_request() {
        let classification = RequestClassifier::classify(
            "Analyze the performance metrics and provide recommendations for optimization",
            &[]
        );
        assert_eq!(classification.complexity, Complexity::Complex);
        assert_eq!(classification.domain, Domain::Analytical);
        assert!(classification.capabilities.contains(&Capability::Reasoning));
    }
    
    #[test]
    fn test_long_context_request() {
        let long_text = "Please review this entire codebase and provide a comprehensive analysis. ".repeat(20);
        let classification = RequestClassifier::classify(&long_text, &[]);
        assert!(classification.capabilities.contains(&Capability::LongContext));
    }
    
    #[test]
    fn test_multiple_images() {
        let images = vec![
            bat_types::message::ImageAttachment {
                data: "".to_string(),
                media_type: "image/png".to_string(),
            },
            bat_types::message::ImageAttachment {
                data: "".to_string(),
                media_type: "image/png".to_string(),
            },
            bat_types::message::ImageAttachment {
                data: "".to_string(),
                media_type: "image/png".to_string(),
            },
        ];
        let classification = RequestClassifier::classify("Analyze these screenshots", &images);
        assert!(classification.capabilities.contains(&Capability::LongContext));
    }
    
    #[test]
    fn test_confidence_calculation() {
        let simple = RequestClassifier::classify("Hello!", &[]);
        let complex = RequestClassifier::classify("Implement a complex algorithm with multiple data structures", &[]);
        
        // Complex requests with clear indicators should have higher confidence
        assert!(complex.confidence >= simple.confidence);
    }
}