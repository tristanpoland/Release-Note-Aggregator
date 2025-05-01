#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use std::collections::HashMap;

    #[test]
    fn test_parse_release_notes() {
        let example_release_notes = r#"# Features

- Added new feature 1
- Added new feature 2

# Bug Fixes

- Fixed bug 1
- Fixed bug 2

# Documentation

- Updated docs"#;

        let sections = parse_release_notes(example_release_notes);
        
        assert_eq!(sections.len(), 3);
        assert!(sections.contains_key("Features"));
        assert!(sections.contains_key("Bug Fixes"));
        assert!(sections.contains_key("Documentation"));
        
        assert_eq!(sections["Features"].len(), 2);
        assert_eq!(sections["Bug Fixes"].len(), 2);
        assert_eq!(sections["Documentation"].len(), 1);
        
        assert_eq!(sections["Features"][0], "- Added new feature 1");
        assert_eq!(sections["Features"][1], "- Added new feature 2");
        assert_eq!(sections["Bug Fixes"][0], "- Fixed bug 1");
        assert_eq!(sections["Bug Fixes"][1], "- Fixed bug 2");
        assert_eq!(sections["Documentation"][0], "- Updated docs");
    }

    #[test]
    fn test_merge_release_notes() {
        // Create mock releases
        let releases = vec![
            Release {
                id: 1,
                tag_name: "v1.0.0".to_string(),
                name: Some("Version 1.0.0".to_string()),
                body: Some(r#"# Features
- Feature A v1
- Feature B v1

# Bug Fixes
- Bug Fix A v1"#.to_string()),
                published_at: "2023-01-01T00:00:00Z".to_string(),
                prerelease: false,
            },
            Release {
                id: 2,
                tag_name: "v2.0.0".to_string(),
                name: Some("Version 2.0.0".to_string()),
                body: Some(r#"# Features
- Feature A v2
- Feature C v2

# Performance
- Performance improvement v2"#.to_string()),
                published_at: "2023-02-01T00:00:00Z".to_string(),
                prerelease: false,
            },
        ];

        let merged_sections = merge_release_notes(&releases);
        
        // Check that we have all expected sections
        assert_eq!(merged_sections.len(), 3);
        assert!(merged_sections.contains_key("Features"));
        assert!(merged_sections.contains_key("Bug Fixes"));
        assert!(merged_sections.contains_key("Performance"));
        
        // Check that the Features section has entries from both releases
        assert_eq!(merged_sections["Features"].len(), 4);
        
        // Check that versions are correctly assigned
        let v1_features = merged_sections["Features"]
            .iter()
            .filter(|item| item.version == "v1.0.0")
            .count();
        
        let v2_features = merged_sections["Features"]
            .iter()
            .filter(|item| item.version == "v2.0.0")
            .count();
        
        assert_eq!(v1_features, 2);
        assert_eq!(v2_features, 2);
        
        // Check that dates are correctly parsed
        let jan_1_2023 = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let feb_1_2023 = NaiveDate::from_ymd_opt(2023, 2, 1).unwrap();
        
        for item in &merged_sections["Features"] {
            if item.version == "v1.0.0" {
                assert_eq!(item.date, jan_1_2023);
            } else if item.version == "v2.0.0" {
                assert_eq!(item.date, feb_1_2023);
            }
        }
    }

    #[test]
    fn test_generate_markdown() {
        let mut merged_sections: HashMap<String, Vec<ReleaseNoteItem>> = HashMap::new();
        
        // Add some test data
        let date1 = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let date2 = NaiveDate::from_ymd_opt(2023, 2, 1).unwrap();
        
        let features = vec![
            ReleaseNoteItem {
                content: "- Feature A v1".to_string(),
                version: "v1.0.0".to_string(),
                date: date1,
            },
            ReleaseNoteItem {
                content: "- Feature B v1".to_string(),
                version: "v1.0.0".to_string(),
                date: date1,
            },
            ReleaseNoteItem {
                content: "- Feature A v2".to_string(),
                version: "v2.0.0".to_string(),
                date: date2,
            },
        ];
        
        let bugs = vec![
            ReleaseNoteItem {
                content: "- Bug Fix A v1".to_string(),
                version: "v1.0.0".to_string(),
                date: date1,
            },
        ];
        
        merged_sections.insert("Features".to_string(), features);
        merged_sections.insert("Bug Fixes".to_string(), bugs);
        
        let markdown = generate_markdown(&merged_sections);
        
        // Check that the markdown contains all expected sections and versions
        assert!(markdown.contains("# Aggregated Release Notes"));
        assert!(markdown.contains("## Bug Fixes"));
        assert!(markdown.contains("## Features"));
        assert!(markdown.contains("### v1.0.0 (2023-01-01)"));
        assert!(markdown.contains("### v2.0.0 (2023-02-01)"));
        
        // Check that content items are included
        assert!(markdown.contains("- Feature A v1"));
        assert!(markdown.contains("- Feature B v1"));
        assert!(markdown.contains("- Feature A v2"));
        assert!(markdown.contains("- Bug Fix A v1"));
    }
}