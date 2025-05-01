# GitHub Release Notes Aggregator - Usage Guide

This document provides detailed examples and usage scenarios for the GitHub Release Notes Aggregator tool.

## Basic Usage

The simplest way to use the tool is to provide the repository owner and name:

```bash
ghnotes --owner microsoft --repo vscode
```

This will fetch all releases from the repository and combine them into a single Markdown file named `aggregated_release_notes.md` in the current directory.

## Filtering by Version Tags

### Using Range of Versions

To aggregate release notes between specific versions:

```bash
ghnotes --owner microsoft --repo vscode --start-tag 1.60.0 --end-tag 1.70.0
```

This will only include releases between these two tags (inclusive).

### Using Arbitrary Versions

To aggregate release notes from specific, non-sequential versions:

```bash
ghnotes --owner microsoft --repo vscode --versions "1.60.0,1.65.0,1.70.0"
```

This will only include the exact versions listed, allowing you to cherry-pick which releases to include in your aggregated notes.

## Merging by Heading

By default, the tool organizes content by section and then by version. To combine similar content across versions under common headings:

```bash
ghnotes --owner microsoft --repo vscode --merge-headings
```

This format is especially useful when you want to see all instances of a particular feature or fix across multiple versions, rather than organizing primarily by version.

You can combine this with version filtering:

```bash
ghnotes --owner microsoft --repo vscode --versions "1.60.0,1.65.0,1.70.0" --merge-headings
```

## Custom Output File

To save the aggregated release notes to a specific file:

```bash
ghnotes --owner microsoft --repo vscode --output ./docs/vscode-releases.md
```

## Including Pre-releases

By default, pre-releases are excluded. To include them:

```bash
ghnotes --owner microsoft --repo vscode --include-prereleases
```

## Using a GitHub Token

GitHub API has rate limits (60 requests per hour for unauthenticated requests). To increase this limit, you can use a GitHub Personal Access Token:

```bash
ghnotes --owner microsoft --repo vscode --token ghp_your_token_here
```

To create a Personal Access Token:
1. Go to GitHub Settings > Developer settings > Personal access tokens
2. Create a new token with the `public_repo` scope (or `repo` for private repositories)

## Combining Options

You can combine multiple options as needed:

```bash
ghnotes --owner microsoft --repo vscode --start-tag 1.60.0 --end-tag 1.70.0 --output ./docs/vscode-releases.md --token ghp_your_token_here --include-prereleases
```

## Output Format

The generated Markdown file is organized by sections found in the release notes, then by version:

```markdown
# Aggregated Release Notes

## Features

### v2.0.0 (2023-05-01)

- Feature A from v2.0.0
- Feature B from v2.0.0

### v1.0.0 (2023-01-01)

- Feature A from v1.0.0
- Feature B from v1.0.0

## Bug Fixes

### v2.0.0 (2023-05-01)

- Bug fix from v2.0.0

### v1.0.0 (2023-01-01)

- Bug fix from v1.0.0
```

## Common Sections in Release Notes

The tool identifies sections based on Markdown headings found in release notes. Common sections include:

- Features
- Bug Fixes/Fixes
- Improvements
- Performance
- Documentation
- Breaking Changes
- Deprecations
- Security
- Uncategorized (for content not under a specific heading)

## Use Cases

### Generating Migration Guides

```bash
ghnotes --owner your-org --repo your-lib --start-tag v1.0.0 --end-tag v2.0.0 --output migration-guide.md
```

This provides a comprehensive view of all changes between major versions, which can be edited to create a migration guide.

### Preparing Release Announcements

```bash
ghnotes --owner your-org --repo your-product --start-tag v1.9.0 --end-tag v2.0.0 --output release-announcement.md
```

This helps in preparing release announcements by aggregating all changes in the upcoming version.

### Creating Changelog for Documentation

```bash
ghnotes --owner your-org --repo your-project --output docs/CHANGELOG.md
```

This creates a complete changelog for your project documentation.

## Troubleshooting

### Rate Limiting

If you see an error about rate limiting, use a GitHub token:

```bash
ghnotes --owner microsoft --repo vscode --token your_github_token
```

### Tag Not Found

If you get a "Tag not found" error, check that:
1. The tag exists in the repository
2. You're using the exact tag name (case-sensitive)
3. The tag has a corresponding release on GitHub

### Empty Sections

If you notice empty sections in your output, this could be because:
1. The release notes don't use consistent Markdown formatting
2. There are no common sections across releases

In these cases, you might need to manually edit the release notes on GitHub to use consistent formatting.

## Advanced Tips

### Automating with CI/CD

You can include the tool in CI/CD pipelines to automatically update your changelog:

```yaml
# GitHub Actions example
steps:
  - uses: actions/checkout@v3
  - name: Update Changelog
    run: |
      cargo install --git https://github.com/yourusername/github-release-notes-aggregator
      ghnotes --owner ${{ github.repository_owner }} --repo ${{ github.event.repository.name }} --output CHANGELOG.md
      git config user.name github-actions
      git config user.email github-actions@github.com
      git add CHANGELOG.md
      git commit -m "Update CHANGELOG.md" || echo "No changes to commit"
      git push
```

### Scripting with jq

You can combine the tool with jq to extract specific information:

```bash
# Get all bug fixes from the last 10 releases
ghnotes --owner microsoft --repo vscode --output - | jq -r '.["Bug Fixes"] | map(.content) | .[]'
```

## Future Improvements

Planned features for future versions:
- Support for GitLab repositories
- HTML output format
- Filtering by date range
- Advanced section merging based on content similarity
- Custom templates for output formatting