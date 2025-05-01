# GitHub Release Notes Aggregator

A command-line tool that aggregates release notes from GitHub repositories between versions, combining them by common sections and saving the result as a Markdown file.

## Features

- Fetch release notes from any public GitHub repository
- Filter releases between specific version tags or select arbitrary versions
- Combine notes by common section headings
- Group release notes by version within each section
- Optional merging of content across versions by common headings
- Optional GitHub token support for higher API rate limits
- Include or exclude pre-releases
- Customizable output file path

## Installation

### Prerequisites

- Rust and Cargo (https://www.rust-lang.org/tools/install)

### Building from source

1. Clone this repository
   ```
   git clone https://github.com/tristanpoland/Release-Note-Aggregator
   cd Release-Note-Agregator
   ```

2. Build with Cargo
   ```
   cargo build --release
   ```

3. The compiled binary will be available at `target/release/ghnotes`

## Usage

```
ghnotes --owner OWNER --repo REPO [OPTIONS]
```

### Required Arguments

- `-o, --owner <OWNER>`: GitHub repository owner (user or organization)
- `-r, --repo <REPO>`: GitHub repository name

### Optional Arguments

- `-s, --start-tag <START_TAG>`: Start tag (older version)
- `-e, --end-tag <END_TAG>`: End tag (newer version)
- `-t, --token <TOKEN>`: GitHub personal access token (for higher rate limits)
- `-o, --output <OUTPUT>`: Output markdown file path (default: `aggregated_release_notes.md`)
- `--include-prereleases`: Include pre-releases (default: false)
- `-h, --help`: Print help
- `-V, --version`: Print version

### Examples

Aggregate all release notes for a repository:
```
ghnotes --owner microsoft --repo vscode
```

Aggregate release notes between specific versions:
```
ghnotes --owner microsoft --repo vscode --start-tag 1.60.0 --end-tag 1.70.0
```

Aggregate release notes for specific, arbitrary versions:
```
ghnotes --owner microsoft --repo vscode --versions "1.60.0,1.65.0,1.70.0"
```

Merge content by common headings instead of grouping by version:
```
ghnotes --owner microsoft --repo vscode --merge-headings
```

Combine arbitrary versions and merge by headings:
```
ghnotes --owner microsoft --repo vscode --versions "1.60.0,1.65.0,1.70.0" --merge-headings
```

Save to a specific file:
```
ghnotes --owner microsoft --repo vscode --output vscode-releases.md
```

Include pre-releases and use a GitHub token:
```
ghnotes --owner microsoft --repo vscode --include-prereleases --token ghp_your_token_here
```

## Output Format

### Standard Format (Default)

The default generated Markdown file is organized by sections, then by versions:

```markdown
# Aggregated Release Notes

## Section Name 1

### v2.0.0 (2023-05-01)

- Feature 1 from v2.0.0
- Feature 2 from v2.0.0

### v1.0.0 (2023-01-01)

- Feature 1 from v1.0.0
- Feature 2 from v1.0.0

## Section Name 2

### v2.0.0 (2023-05-01)

- Another feature from v2.0.0

### v1.0.0 (2023-01-01)

- Another feature from v1.0.0
```

### Merged Headings Format (with `--merge-headings` flag)

When using the `--merge-headings` flag, the output is organized by sections, with similar content merged across versions:

```markdown
# Aggregated Release Notes (Merged by Heading)

## Section Name 1

- Feature that exists in multiple versions
*(Present in versions: v1.0.0, v2.0.0)*

- Feature unique to one version
*(From version: v2.0.0)*

## Section Name 2

- Another feature that appears across versions
*(Present in versions: v1.0.0, v2.0.0)*
```

This merged format is especially useful when you want to see the complete set of features or fixes across multiple versions without duplication.

## Limitations

- GitHub API has rate limits (60 requests per hour for unauthenticated requests)
- Only fetches up to 100 most recent releases by default
- Requires proper Markdown headings in release notes for section separation

## License

This project is licensed under the MIT License - see the LICENSE file for details.
