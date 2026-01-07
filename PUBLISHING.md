# Publishing rbuster to crates.io

This guide explains how to use the automated CI/CD pipeline to build, release, and publish rbuster.

## Setup Steps

### 1. Get Crates.io API Token
- Go to https://crates.io and create an account (or sign in)
- Navigate to Account Settings → API Tokens
- Generate a new token
- Copy the token

### 2. Add Secret to GitHub Repository
- Go to your GitHub repository: https://github.com/R0h1tAnand/rbuster
- Settings → Secrets and variables → Actions
- Click "New repository secret"
- Name: `CARGO_REGISTRY_TOKEN`
- Value: (paste the token from crates.io)
- Click "Add secret"

### 3. Verify Cargo.toml (Already Done ✓)
Your Cargo.toml has all required fields:
- `name = "rbuster"`
- `version = "1.0.0"`
- `description = "..."`
- `license = "MIT"`
- `repository = "https://github.com/R0h1tAnand/rbuster"`

## Workflow

### Step 1: Develop and Test Locally
```bash
cargo test
cargo fmt
cargo clippy
```

### Step 2: Commit and Push to main
```bash
git add .
git commit -m "Your changes"
git push origin main
```
GitHub Actions will automatically run CI (tests, formatting, clippy).

### Step 3: Create a Release Tag
When ready to release, create a version tag:
```bash
git tag v1.1.0
git push origin v1.1.0
```

This triggers:
1. **release.yml** - Builds the binary and creates a GitHub Release
2. **publish.yml** - Publishes to crates.io automatically

### Manual Publishing (Alternative)
If you prefer manual control:
```bash
# Update version in Cargo.toml
# e.g., from 1.0.0 to 1.1.0

cargo publish --token <your-token>
```

## What Each Workflow Does

### ci.yml (Continuous Integration)
- Runs on every push and PR
- Tests on Linux, macOS, and Windows
- Checks code formatting (rustfmt)
- Runs Clippy linter
- Tests with stable and beta Rust

### release.yml (GitHub Release)
- Triggers when you push a version tag (v*)
- Builds the release binary
- Creates a GitHub Release with the binary as attachment

### publish.yml (Crates.io Publishing)
- Triggers when a GitHub Release is published
- Automatically publishes to crates.io
- Users can then install with: `cargo install rbuster`

## Troubleshooting

### "Publishing failed" error
- Verify CARGO_REGISTRY_TOKEN is correct
- Check that version in Cargo.toml is unique (higher than previous)
- Ensure crates.io metadata is correct (description, license, etc.)

### Version already exists
- Update version in Cargo.toml before creating tag
- Version must be higher than the last published version

## Installation Command (After Publishing)
Users can install rbuster with:
```bash
cargo install rbuster
```

## Versioning Convention
Follow semantic versioning:
- v1.0.0 (major.minor.patch)
- v1.0.1 - patch release (bug fixes)
- v1.1.0 - minor release (new features)
- v2.0.0 - major release (breaking changes)
