# 🚀 Quick Start: Automated Releases

## How to Release a New Version

**It's super simple! Just 3 steps:**

### 1️⃣ Update the version in Cargo.toml

Open `Cargo.toml` and change the version:

```toml
[package]
name = "robuster"
version = "1.0.1"  # <-- Change this number
```

### 2️⃣ Commit and push

```bash
git add Cargo.toml
git commit -m "chore: bump version to 1.0.1"
git push origin main
```

### 3️⃣ Done! 🎉

The GitHub Actions workflow will automatically:
- ✅ Detect the new version from Cargo.toml
- ✅ Check if it already exists (skips if duplicate)
- ✅ Run all tests
- ✅ Build Linux and Windows binaries
- ✅ Create a git tag (e.g., v1.0.1)
- ✅ Create a GitHub Release with binaries
- ✅ Publish to crates.io

## What You Don't Need to Do

❌ No manual git tags  
❌ No manual releases  
❌ No manual building  
❌ No manual publishing  

The workflow handles everything!

## Check the Status

- **Workflow runs**: https://github.com/R0h1tAnand/robuster/actions
- **Releases**: https://github.com/R0h1tAnand/robuster/releases
- **Crates.io**: https://crates.io/crates/robuster

## Example

```bash
# Current version in Cargo.toml: 1.0.0
# Want to release 1.0.1

# 1. Edit Cargo.toml
vim Cargo.toml  # Change version = "1.0.1"

# 2. Commit and push
git add Cargo.toml
git commit -m "chore: bump to 1.0.1"
git push origin main

# 3. Watch it happen automatically!
# Visit: https://github.com/R0h1tAnand/robuster/actions
```

## How It Determines What to Build

The workflow checks:
1. **Did Cargo.toml change?** → Yes, check version
2. **Is this version new?** → Compare with existing git tags
3. **New version found?** → Run full release process
4. **Version exists?** → Skip (prevents duplicates)

## Smart Duplicate Prevention

If you accidentally push the same version twice:
- The workflow detects the existing tag
- Skips the build and release
- No errors, just a friendly skip message

## Troubleshooting

**Q: The workflow didn't run**  
A: Check that you pushed to the `main` branch and modified `Cargo.toml` or files in `src/`

**Q: How do I see what's happening?**  
A: Go to https://github.com/R0h1tAnand/robuster/actions and click on the latest "Auto Release & Publish" workflow

**Q: Can I test without releasing?**  
A: Yes! Push to a different branch (not `main`). The workflow only runs on `main`.

**Q: What if the crates.io publish fails?**  
A: The workflow continues anyway (won't fail the whole process). You can manually publish with: `cargo publish`
