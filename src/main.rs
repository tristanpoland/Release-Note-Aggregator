use anyhow::{Context, Result};
use chrono::NaiveDate;
use clap::Parser;
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
// Added for logging
use log::{debug, info, warn, error};
use env_logger;

#[derive(Parser, Debug)]
#[command(
    name = "github-release-notes-aggregator",
    about = "Aggregates GitHub release notes between versions",
    version,
    author
)]
struct Cli {
    /// GitHub repository owner (user or organization)
    #[arg(short, long)]
    owner: String,

    /// GitHub repository name
    #[arg(short, long)]
    repo: String,

    /// Start tag (older version)
    #[arg(short, long)]
    start_tag: Option<String>,

    /// End tag (newer version)
    #[arg(short, long)]
    end_tag: Option<String>,

    /// GitHub personal access token (for higher rate limits)
    #[arg(short, long)]
    token: Option<String>,

    /// Output markdown file path
    #[arg(long, default_value = "aggregated_release_notes.md")]
    output: PathBuf,

    /// Include pre-releases
    #[arg(long, default_value = "false")]
    include_prereleases: bool,

    /// Arbitrary versions to merge (comma-separated list of tag names)
    #[arg(short = 'v', long)]
    versions: Option<String>,

    /// Merge by heading (combine content under common headings instead of keeping versions separate)
    #[arg(short = 'm', long, default_value = "false")]
    merge_headings: bool,
    
    /// Enable verbose logging
    #[arg(long, default_value = "false")]
    verbose: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Release {
    id: u64,
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    published_at: String,
    prerelease: bool,
}

#[derive(Debug)]
struct ReleaseNoteItem {
    content: String,
    version: String,
    date: NaiveDate,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logger
    if cli.verbose {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    } else {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }
    
    info!("Fetching release notes for {}/{}", cli.owner, cli.repo);

    // Get all releases first
    let all_releases = fetch_all_releases(&cli).await?;
    info!("Found {} releases total", all_releases.len());

    if all_releases.is_empty() {
        warn!("No releases found. Exiting.");
        return Ok(());
    }

    // Determine which releases to process based on CLI flags
    let releases_to_process = if let Some(versions) = &cli.versions {
        // Process arbitrary versions
        let version_tags: Vec<&str> = versions.split(',').map(|s| s.trim()).collect();
        debug!("Processing specific versions: {:?}", version_tags);
        filter_releases_by_tags(&all_releases, &version_tags)?
    } else if cli.start_tag.is_some() || cli.end_tag.is_some() {
        // Process range of versions
        debug!("Processing range: start={:?}, end={:?}", cli.start_tag, cli.end_tag);
        filter_releases_by_range(&all_releases, cli.start_tag.as_deref(), cli.end_tag.as_deref())?
    } else {
        // Process all releases
        debug!("Processing all releases");
        all_releases
    };

    info!("Processing {} releases", releases_to_process.len());

    let markdown = if cli.merge_headings {
        // Merge content under common headings
        debug!("Merging release notes by heading");
        let merged_by_heading = merge_release_notes_by_heading(&releases_to_process);
        generate_markdown_merged_headings(&merged_by_heading)
    } else {
        // Traditional merge - keep versions separate under each heading
        debug!("Merging release notes by version");
        let merged_sections = merge_release_notes(&releases_to_process);
        generate_markdown(&merged_sections)
    };

    // Write to file
    debug!("Writing output to {:?}", cli.output);
    let mut file = File::create(&cli.output)
        .with_context(|| format!("Failed to create output file: {:?}", cli.output))?;
    file.write_all(markdown.as_bytes())
        .with_context(|| format!("Failed to write to output file: {:?}", cli.output))?;

    info!("Successfully wrote aggregated release notes to {:?}", cli.output);
    Ok(())
}

async fn fetch_all_releases(cli: &Cli) -> Result<Vec<Release>> {
    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("github-release-notes-aggregator"));
    
    if let Some(token) = &cli.token {
        debug!("Using GitHub personal access token for authentication");
        headers.insert(
            reqwest::header::AUTHORIZATION,
            HeaderValue::from_str(&format!("token {}", token))?,
        );
    } else {
        debug!("No GitHub token provided, using unauthenticated requests");
    }

    let url = format!(
        "https://api.github.com/repos/{}/{}/releases?per_page=100",
        cli.owner, cli.repo
    );
    
    info!("Making API request to: {}", url);
    
    // Log request details before sending
    debug!("API Request: GET {}", url);
    debug!("Headers: {:?}", headers);
    
    let response = client
        .get(&url)
        .headers(headers)
        .send()
        .await
        .context("Failed to send request to GitHub API")?;
    
    // Log response details
    debug!("API Response: Status: {}", response.status());
    debug!("Response headers: {:?}", response.headers());
    
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_else(|_| "Unable to read response body".to_string());
        error!("GitHub API error: Status={}, Body={}", status, body);
        return Err(anyhow::anyhow!(
            "GitHub API returned error status: {}, Body: {}",
            status, body
        ));
    }
    
    // Clone the response for logging the body if needed
    let response_text = response.text().await.context("Failed to get response text")?;
    debug!("Response body length: {} bytes", response_text.len());
    
    if cli.verbose {
        debug!("First 500 characters of response: {}", 
            if response_text.len() > 500 {
                &response_text[..500]
            } else {
                &response_text
            }
        );
    }
    
    // Parse the JSON response
    let releases: Vec<Release> = serde_json::from_str(&response_text)
        .context("Failed to parse GitHub API response")?;
    
    debug!("Parsed {} releases from API response", releases.len());

    // Filter out prereleases if not included
    let filtered_releases = if !cli.include_prereleases {
        let prerelease_count = releases.iter().filter(|r| r.prerelease).count();
        let filtered = releases.into_iter().filter(|r| !r.prerelease).collect::<Vec<_>>();
        debug!("Filtered out {} prereleases", prerelease_count);
        filtered
    } else {
        releases
    };

    // Sort by published date (newest first)
    let mut sorted_releases = filtered_releases;
    sorted_releases.sort_by(|a, b| {
        let date_a = chrono::DateTime::parse_from_rfc3339(&a.published_at)
            .unwrap()
            .naive_utc();
        let date_b = chrono::DateTime::parse_from_rfc3339(&b.published_at)
            .unwrap()
            .naive_utc();
        date_b.cmp(&date_a)
    });
    
    debug!("Releases sorted by date (newest first)");

    Ok(sorted_releases)
}

fn filter_releases_by_range(
    releases: &[Release], 
    start_tag: Option<&str>,
    end_tag: Option<&str>
) -> Result<Vec<Release>> {
    let mut filtered = releases.to_vec();
    
    if let (Some(start_tag), Some(end_tag)) = (start_tag, end_tag) {
        debug!("Filtering releases between tags '{}' and '{}'", start_tag, end_tag);
        let start_index = releases
            .iter()
            .position(|r| r.tag_name == start_tag)
            .context(format!("Start tag '{}' not found", start_tag))?;
        
        let end_index = releases
            .iter()
            .position(|r| r.tag_name == end_tag)
            .context(format!("End tag '{}' not found", end_tag))?;

        // Ensure we get releases between the two tags (inclusive)
        let (lower_index, higher_index) = if start_index <= end_index {
            (start_index, end_index)
        } else {
            (end_index, start_index)
        };
        
        debug!("Tag indices: start={}, end={}", lower_index, higher_index);

        filtered = releases.iter().enumerate()
            .filter(|(i, _)| *i >= lower_index && *i <= higher_index)
            .map(|(_, r)| r.clone())
            .collect();
    } else if let Some(start_tag) = start_tag {
        // Only start tag specified - get from that tag to the latest
        debug!("Filtering releases from tag '{}' to latest", start_tag);
        let start_index = releases
            .iter()
            .position(|r| r.tag_name == start_tag)
            .context(format!("Start tag '{}' not found", start_tag))?;
            
        filtered = releases.iter().enumerate()
            .filter(|(i, _)| *i >= start_index)
            .map(|(_, r)| r.clone())
            .collect();
    } else if let Some(end_tag) = end_tag {
        // Only end tag specified - get from the earliest to that tag
        debug!("Filtering releases from earliest to tag '{}'", end_tag);
        let end_index = releases
            .iter()
            .position(|r| r.tag_name == end_tag)
            .context(format!("End tag '{}' not found", end_tag))?;
            
        filtered = releases.iter().enumerate()
            .filter(|(i, _)| *i <= end_index)
            .map(|(_, r)| r.clone())
            .collect();
    }
    
    info!("Filtered to {} releases", filtered.len());
    Ok(filtered)
}

fn filter_releases_by_tags(releases: &[Release], tags: &[&str]) -> Result<Vec<Release>> {
    debug!("Filtering releases by specific tags: {:?}", tags);
    let mut filtered_releases = Vec::new();
    let mut missing_tags = Vec::new();
    
    for tag in tags {
        let release = releases.iter().find(|r| &r.tag_name == tag);
        
        match release {
            Some(release) => filtered_releases.push(release.clone()),
            None => missing_tags.push(*tag),
        }
    }
    
    if !missing_tags.is_empty() {
        error!("Missing tags: {:?}", missing_tags);
        return Err(anyhow::anyhow!(
            "Could not find the following tags: {}",
            missing_tags.join(", ")
        ));
    }
    
    // Sort by published date (newest first)
    filtered_releases.sort_by(|a, b| {
        let date_a = chrono::DateTime::parse_from_rfc3339(&a.published_at)
            .unwrap()
            .naive_utc();
        let date_b = chrono::DateTime::parse_from_rfc3339(&b.published_at)
            .unwrap()
            .naive_utc();
        date_b.cmp(&date_a)
    });
    
    info!("Filtered to {} releases", filtered_releases.len());
    Ok(filtered_releases)
}

fn parse_release_notes(body: &str) -> HashMap<String, Vec<String>> {
    let mut sections: HashMap<String, Vec<String>> = HashMap::new();
    let mut current_section = "Uncategorized".to_string();
    
    // Initialize with uncategorized section
    sections.insert(current_section.clone(), Vec::new());
    
    // Define a regex for Markdown headings
    let heading_regex = Regex::new(r"^(#{1,6})\s+(.+)$").unwrap();
    
    for line in body.lines() {
        if let Some(captures) = heading_regex.captures(line) {
            current_section = captures.get(2).unwrap().as_str().trim().to_string();
            if !sections.contains_key(&current_section) {
                sections.insert(current_section.clone(), Vec::new());
            }
        } else if !line.trim().is_empty() {
            // Add non-empty lines to the current section
            sections.get_mut(&current_section).unwrap().push(line.to_string());
        }
    }
    
    // Remove sections with no content
    sections.retain(|_, lines| !lines.is_empty());
    
    debug!("Parsed {} sections from release notes", sections.len());
    sections
}

fn merge_release_notes(releases: &[Release]) -> HashMap<String, Vec<ReleaseNoteItem>> {
    let mut merged_sections: HashMap<String, Vec<ReleaseNoteItem>> = HashMap::new();
    let mut known_sections: HashSet<String> = HashSet::new();
    
    // First pass - collect all possible sections
    for release in releases {
        if let Some(body) = &release.body {
            let sections = parse_release_notes(body);
            for section_name in sections.keys() {
                known_sections.insert(section_name.clone());
            }
        }
    }
    
    debug!("Found {} unique section names across all releases", known_sections.len());
    
    // Initialize merged sections
    for section in known_sections {
        merged_sections.insert(section, Vec::new());
    }
    
    // Second pass - populate sections with items
    for release in releases {
        if let Some(body) = &release.body {
            let version = release.tag_name.clone();
            let date = chrono::DateTime::parse_from_rfc3339(&release.published_at)
                .unwrap()
                .naive_utc()
                .date();
            
            debug!("Processing release {} ({})", version, date);
            let sections = parse_release_notes(body);
            
            for (section_name, items) in sections {
                for item in items {
                    let note_item = ReleaseNoteItem {
                        content: item,
                        version: version.clone(),
                        date,
                    };
                    
                    merged_sections.get_mut(&section_name).unwrap().push(note_item);
                }
            }
        } else {
            debug!("Release {} has no body content", release.tag_name);
        }
    }
    
    debug!("Merged all release notes into sections");
    merged_sections
}

// New function for merging content under common headings
#[derive(Debug)]
struct MergedHeadingItem {
    content: String,
    sources: Vec<String>, // List of versions this item came from
}

fn merge_release_notes_by_heading(releases: &[Release]) -> HashMap<String, Vec<MergedHeadingItem>> {
    let mut merged_sections: HashMap<String, Vec<MergedHeadingItem>> = HashMap::new();
    let mut known_sections: HashSet<String> = HashSet::new();
    
    // First pass - collect all possible sections
    for release in releases {
        if let Some(body) = &release.body {
            let sections = parse_release_notes(body);
            for section_name in sections.keys() {
                known_sections.insert(section_name.clone());
            }
        }
    }
    
    debug!("Found {} unique section names across all releases", known_sections.len());
    
    // Initialize merged sections
    for section in known_sections {
        merged_sections.insert(section, Vec::new());
    }
    
    // Second pass - collect all content items by section
    let mut content_map: HashMap<String, HashMap<String, Vec<String>>> = HashMap::new();
    
    for release in releases {
        if let Some(body) = &release.body {
            let version = release.tag_name.clone();
            debug!("Processing release {} for heading merge", version);
            let sections = parse_release_notes(body);
            
            for (section_name, items) in sections {
                if !content_map.contains_key(&section_name) {
                    content_map.insert(section_name.clone(), HashMap::new());
                }
                
                let section_content = content_map.get_mut(&section_name).unwrap();
                
                for item in items {
                    // Normalize the content by trimming whitespace
                    let normalized_content = item.trim().to_string();
                    
                    if !section_content.contains_key(&normalized_content) {
                        section_content.insert(normalized_content.clone(), Vec::new());
                    }
                    
                    section_content.get_mut(&normalized_content).unwrap().push(version.clone());
                }
            }
        }
    }
    
    // Third pass - create merged items
    for (section_name, content_items) in content_map {
        let mut merged_items = Vec::new();
        
        for (content, versions) in content_items {
            let merged_item = MergedHeadingItem {
                content,
                sources: versions,
            };
            
            merged_items.push(merged_item);
        }
        
        // Sort items by how many versions they appear in (most common first)
        merged_items.sort_by(|a, b| {
            // First by number of sources (descending)
            let source_cmp = b.sources.len().cmp(&a.sources.len());
            
            // Then alphabetically by content if tied
            if source_cmp == std::cmp::Ordering::Equal {
                a.content.cmp(&b.content)
            } else {
                source_cmp
            }
        });
        
        merged_sections.insert(section_name, merged_items);
    }
    
    debug!("Merged release notes by heading");
    merged_sections
}

fn generate_markdown(
    merged_sections: &HashMap<String, Vec<ReleaseNoteItem>>,
) -> String {
    debug!("Generating markdown output (version-based)");
    let mut markdown = String::from("# Aggregated Release Notes\n\n");
    
    // Sort sections alphabetically, but put "Uncategorized" at the end
    let mut section_names: Vec<&String> = merged_sections.keys().collect();
    section_names.sort_by(|a, b| {
        if *a == "Uncategorized" {
            std::cmp::Ordering::Greater
        } else if *b == "Uncategorized" {
            std::cmp::Ordering::Less
        } else {
            a.cmp(b)
        }
    });
    
    for section_name in section_names {
        debug!("Processing section: {}", section_name);
        markdown.push_str(&format!("## {}\n\n", section_name));
        
        let items = &merged_sections[section_name];
        
        // Group items by version
        let mut versions = HashMap::new();
        for item in items {
            versions
                .entry((item.version.clone(), item.date))
                .or_insert_with(Vec::new)
                .push(item);
        }
        
        // Sort versions by date (newest first)
        let mut version_entries: Vec<_> = versions.into_iter().collect();
        version_entries.sort_by(|a, b| b.0.1.cmp(&a.0.1));
        
        for ((version, date), version_items) in version_entries {
            debug!("Adding version: {} ({})", version, date);
            markdown.push_str(&format!(
                "### {} ({})\n\n",
                version,
                date.format("%Y-%m-%d")
            ));
            
            for item in version_items {
                markdown.push_str(&format!("{}\n", item.content));
            }
            
            markdown.push_str("\n");
        }
    }
    
    info!("Generated markdown output: {} bytes", markdown.len());
    markdown
}

// New function to generate markdown with merged headings
fn generate_markdown_merged_headings(
    merged_sections: &HashMap<String, Vec<MergedHeadingItem>>,
) -> String {
    debug!("Generating markdown output (heading-based)");
    let mut markdown = String::from("# Aggregated Release Notes (Merged by Heading)\n\n");
    
    // Sort sections alphabetically, but put "Uncategorized" at the end
    let mut section_names: Vec<&String> = merged_sections.keys().collect();
    section_names.sort_by(|a, b| {
        if *a == "Uncategorized" {
            std::cmp::Ordering::Greater
        } else if *b == "Uncategorized" {
            std::cmp::Ordering::Less
        } else {
            a.cmp(b)
        }
    });
    
    for section_name in section_names {
        debug!("Processing section: {}", section_name);
        markdown.push_str(&format!("## {}\n\n", section_name));
        
        let items = &merged_sections[section_name];
        
        for item in items {
            // Add the content
            markdown.push_str(&format!("{}\n", item.content));
            
            // Add source versions if there are multiple
            if item.sources.len() > 1 {
                let sorted_sources = {
                    let mut sources = item.sources.clone();
                    sources.sort();
                    sources
                };
                
                let sources_list = sorted_sources.join(", ");
                debug!("Item appears in multiple versions: {}", sources_list);
                markdown.push_str(&format!("*(Present in versions: {})*\n\n", sources_list));
            } else if !item.sources.is_empty() {
                debug!("Item appears in single version: {}", item.sources[0]);
                markdown.push_str(&format!("*(From version: {})*\n\n", item.sources[0]));
            } else {
                markdown.push_str("\n");
            }
        }
        
        markdown.push_str("\n");
    }
    
    info!("Generated markdown output: {} bytes", markdown.len());
    markdown
}