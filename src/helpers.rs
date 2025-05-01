use anyhow::{Context, Result};
use chrono::NaiveDate;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;

/// Helper struct for parsing GitHub rate limit information
#[derive(Debug, Deserialize)]
pub struct RateLimit {
    pub limit: u32,
    pub remaining: u32,
    pub reset: u64,
}

/// Helper struct for parsing GitHub API error responses
#[derive(Debug, Deserialize)]
pub struct GitHubError {
    pub message: String,
    pub documentation_url: Option<String>,
}

/// Parse GitHub rate limit information from response headers
pub fn parse_rate_limit(headers: &reqwest::header::HeaderMap) -> Option<RateLimit> {
    let limit = headers
        .get("x-ratelimit-limit")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u32>().ok());
    
    let remaining = headers
        .get("x-ratelimit-remaining")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u32>().ok());
    
    let reset = headers
        .get("x-ratelimit-reset")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok());
    
    if let (Some(limit), Some(remaining), Some(reset)) = (limit, remaining, reset) {
        Some(RateLimit {
            limit,
            remaining,
            reset,
        })
    } else {
        None
    }
}

/// Format a timestamp as a human-readable date
pub fn format_date(date_str: &str) -> Result<String> {
    let date = chrono::DateTime::parse_from_rfc3339(date_str)
        .context("Failed to parse date")?
        .naive_utc()
        .date();
    
    Ok(date.format("%Y-%m-%d").to_string())
}

/// Extract version number from tag name (e.g., "v1.2.3" -> "1.2.3")
pub fn extract_version(tag_name: &str) -> String {
    let re = Regex::new(r"^[vV]?(.+)$").unwrap();
    if let Some(caps) = re.captures(tag_name) {
        caps.get(1).unwrap().as_str().to_string()
    } else {
        tag_name.to_string()
    }
}

/// Normalize section name for consistent matching
pub fn normalize_section_name(name: &str) -> String {
    name.trim().to_lowercase()
}

/// Group items by section and version
pub fn group_by_section_and_version(
    items: Vec<(String, String, String, NaiveDate)>,
) -> HashMap<String, HashMap<(String, NaiveDate), Vec<String>>> {
    let mut result: HashMap<String, HashMap<(String, NaiveDate), Vec<String>>> = HashMap::new();
    
    for (section, content, version, date) in items {
        result
            .entry(section)
            .or_insert_with(HashMap::new)
            .entry((version, date))
            .or_insert_with(Vec::new)
            .push(content);
    }
    
    result
}

/// Clean up markdown content by removing extra blank lines and ensuring proper spacing
pub fn clean_markdown(content: &str) -> String {
    // Remove multiple consecutive blank lines
    let re = Regex::new(r"\n{3,}").unwrap();
    let content = re.replace_all(content, "\n\n").to_string();
    
    // Ensure headings are preceded by a blank line (except at the start)
    let re = Regex::new(r"(?m)^(?!#)(.+)\n(#+\s)").unwrap();
    let content = re.replace_all(&content, "$1\n\n$2").to_string();
    
    content
}

/// Extract sections from Markdown content
pub fn extract_sections(content: &str) -> HashMap<String, Vec<String>> {
    let mut sections = HashMap::new();
    let heading_regex = Regex::new(r"^(#+)\s+(.+)$").unwrap();
    
    let mut current_section = "Uncategorized".to_string();
    let mut current_level = 0;
    let mut current_content = Vec::new();
    
    for line in content.lines() {
        if let Some(captures) = heading_regex.captures(line) {
            let level = captures.get(1).unwrap().as_str().len();
            let heading = captures.get(2).unwrap().as_str().trim();
            
            // Only consider top-level and second-level headings as section dividers
            if level <= 2 {
                // Save the previous section
                if !current_content.is_empty() {
                    sections.insert(current_section, current_content);
                }
                
                // Start a new section
                current_section = heading.to_string();
                current_level = level;
                current_content = Vec::new();
            } else {
                // For deeper headings, include them in the content
                current_content.push(line.to_string());
            }
        } else {
            current_content.push(line.to_string());
        }
    }
    
    // Save the last section
    if !current_content.is_empty() {
        sections.insert(current_section, current_content);
    }
    
    sections
}

/// Check if a tag follows semantic versioning
pub fn is_semver(tag: &str) -> bool {
    let tag = if tag.starts_with('v') || tag.starts_with('V') {
        &tag[1..]
    } else {
        tag
    };
    
    let re = Regex::new(r"^(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$").unwrap();
    re.is_match(tag)
}

/// Compare two semantic version tags
pub fn compare_semver(tag1: &str, tag2: &str) -> std::cmp::Ordering {
    let clean1 = extract_version(tag1);
    let clean2 = extract_version(tag2);
    
    if !is_semver(&clean1) || !is_semver(&clean2) {
        // Fall back to string comparison if not semver
        return clean1.cmp(&clean2);
    }
    
    let v1: Vec<&str> = clean1.split('.').collect();
    let v2: Vec<&str> = clean2.split('.').collect();
    
    for i in 0..3 {
        if i >= v1.len() || i >= v2.len() {
            return v1.len().cmp(&v2.len());
        }
        
        let n1 = v1[i].parse::<u32>().unwrap_or(0);
        let n2 = v2[i].parse::<u32>().unwrap_or(0);
        
        match n1.cmp(&n2) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }
    
    std::cmp::Ordering::Equal
}