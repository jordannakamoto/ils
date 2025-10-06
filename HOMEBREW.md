# Homebrew Submission Guide for ils

## Current Status

✅ **Ready for Homebrew submission**

- GitHub repository: https://github.com/jordannakamoto/ils
- Latest release: v0.1.1
- License: MIT
- Platform: macOS only (uses macOS-specific features)

## Formula Location

The Homebrew formula is located at `ils.rb` in the project root.

## Installation Methods

### Option 1: Create Your Own Tap (Recommended for testing)

```bash
# Create a Homebrew tap
brew tap-new jordannakamoto/tap

# Copy the formula to your tap
cp ils.rb $(brew --repository)/Library/Taps/jordannakamoto/homebrew-tap/Formula/

# Install from your tap
brew install jordannakamoto/tap/ils

# Don't forget to add shell integration:
echo 'source $(brew --prefix)/opt/ils/etc/ils_init.sh' >> ~/.zshrc
source ~/.zshrc
```

### Option 2: Submit to Homebrew Core (Public distribution)

**Prerequisites:**
- Stable release with version tag
- Working formula
- 75+ stars or 30+ forks on GitHub (for homebrew-core)
- Formula follows [Homebrew guidelines](https://docs.brew.sh/Formula-Cookbook)

**Submission Process:**

1. **Test the formula locally:**
   ```bash
   brew install --build-from-source ils.rb
   brew test ils
   brew audit --new ils.rb
   ```

2. **Create a tap for initial distribution:**
   ```bash
   # Create homebrew-tap repository on GitHub
   gh repo create homebrew-tap --public

   # Set up tap structure
   mkdir -p Formula
   cp ils.rb Formula/
   git add Formula/ils.rb
   git commit -m "Add ils formula"
   git push
   ```

3. **Users can install from your tap:**
   ```bash
   brew tap jordannakamoto/tap
   brew install ils
   ```

4. **Submit to Homebrew Core (when ready):**
   - Formula must be in a tap for at least 30 days
   - Project should have significant adoption
   - Open PR to [Homebrew/homebrew-core](https://github.com/Homebrew/homebrew-core)

## Formula Validation

Run these checks before submission:

```bash
# Build from source
brew install --build-from-source ./ils.rb

# Run tests
brew test ils

# Audit formula
brew audit --new --strict ils

# Check for common issues
brew style ils
```

## Post-Installation

Users need to source the shell integration:

```bash
# Add to ~/.zshrc or ~/.bashrc
source $(brew --prefix)/opt/ils/etc/ils_init.sh
```

Or run the helper:
```bash
echo 'source $(brew --prefix)/opt/ils/etc/ils_init.sh' >> ~/.zshrc
```

## Updating the Formula

When releasing a new version:

1. **Create a new GitHub release:**
   ```bash
   git tag v0.1.2
   git push origin v0.1.2
   gh release create v0.1.2 --generate-notes
   ```

2. **Update the formula:**
   ```bash
   # Get new SHA256
   curl -sL https://github.com/jordannakamoto/ils/archive/refs/tags/v0.1.2.tar.gz | shasum -a 256

   # Update ils.rb:
   # - url: change version number
   # - sha256: new hash from above
   ```

3. **Test the updated formula:**
   ```bash
   brew reinstall ./ils.rb
   brew test ils
   ```

4. **Push updates:**
   ```bash
   git add ils.rb
   git commit -m "Update formula to v0.1.2"
   git push
   ```

## Troubleshooting

### "Formula not found"
Make sure you're in the project directory or use the full path to `ils.rb`.

### "Checksum mismatch"
Regenerate the SHA256:
```bash
curl -sL https://github.com/jordannakamoto/ils/archive/refs/tags/v0.1.1.tar.gz | shasum -a 256
```

### "Permission denied" when installing
The formula correctly installs as non-root. If you see permission errors, check your Homebrew installation:
```bash
brew doctor
```

### Shell integration not working
Make sure you've sourced the init script and restarted your terminal:
```bash
source ~/.zshrc  # or ~/.bashrc
```

## Resources

- [Homebrew Formula Cookbook](https://docs.brew.sh/Formula-Cookbook)
- [Homebrew Acceptable Formulae](https://docs.brew.sh/Acceptable-Formulae)
- [Creating Taps](https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap)
- [Submitting to Core](https://docs.brew.sh/How-To-Open-a-Homebrew-Pull-Request)

## Checklist for Homebrew Core Submission

- [ ] Project has 75+ stars or 30+ forks
- [ ] Formula in personal tap for 30+ days
- [ ] Formula passes `brew audit --strict`
- [ ] Formula passes `brew test`
- [ ] Builds successfully on latest macOS
- [ ] License is clearly documented (MIT)
- [ ] README has clear installation instructions
- [ ] No major open issues or bugs
- [ ] Active maintenance (recent commits)
- [ ] macOS-only designation is clear

## Next Steps

1. ✅ Formula is ready and tested
2. ✅ GitHub release v0.1.1 is published
3. **Create homebrew-tap repository** (optional, for distribution)
4. **Build community** (stars, usage, feedback)
5. **Submit to Homebrew Core** (when criteria met)

---

**Note:** Start with a personal tap for easier distribution. Submit to Homebrew Core once the project has gained traction and meets all requirements.
