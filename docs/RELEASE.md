# Release Process

This document describes the automated release process for the Locai project.

## Automated Release Pipeline

We use [release-please](https://github.com/googleapis/release-please) for fully automated releases:

### How It Works

1. **Conventional Commits**: Developers use conventional commit messages
2. **Release PR Creation**: release-please analyzes commits and creates a release PR with:
   - Version bumps in all `Cargo.toml` files
   - Updated `CHANGELOG.md` 
   - Git tags preparation
3. **Review & Merge**: Maintainers review and merge the release PR
4. **Automatic Publishing**: GitHub Actions publishes all crates to crates.io

### Configuration Files

- `release-please-config.json`: Main configuration for the release process
- `.release-please-manifest.json`: Tracks current versions of all packages
- `.github/workflows/release.yml`: GitHub Actions workflow for publishing

## Release Checklist

### Before Release

- [ ] All CI checks passing
- [ ] Documentation up to date
- [ ] Examples working correctly
- [ ] Breaking changes documented
- [ ] Security vulnerabilities addressed

### Release Process

1. **Wait for Release PR**: release-please automatically creates a release PR
2. **Review Changes**: Check the generated changelog and version bumps
3. **Test Release**: Verify builds and tests pass
4. **Merge PR**: Merge the release PR to trigger publishing
5. **Verify Publication**: Check that packages appear on crates.io

### Post-Release

- [ ] Verify crates.io publication
- [ ] Test installation: `cargo install locai-cli`
- [ ] Update documentation sites if needed
- [ ] Announce release in appropriate channels

## Manual Release Process

If automation fails, manual release process:

### 1. Version Bumps

```bash
# Update versions in workspace
cargo install cargo-edit
cargo set-version --workspace 0.2.0
```

### 2. Update Changelog

```bash
# Add new version section to CHANGELOG.md
# Follow Keep a Changelog format
```

### 3. Create Git Tag

```bash
git add .
git commit -m "chore: release v0.2.0"
git tag v0.2.0
git push origin main --tags
```

### 4. Publish Crates

```bash
# Publish in dependency order
cd locai && cargo publish
sleep 60  # Wait for crates.io processing
cd ../locai-server && cargo publish  
cd ../locai-cli && cargo publish
```

### 5. Create GitHub Release

- Go to GitHub Releases
- Create new release from tag
- Copy changelog content
- Mark as prerelease if alpha/beta

## Version Strategy

### Semantic Versioning

We follow [SemVer](https://semver.org/):

- `MAJOR`: Breaking changes (1.0.0 → 2.0.0)
- `MINOR`: New features, backward compatible (1.0.0 → 1.1.0)
- `PATCH`: Bug fixes, backward compatible (1.0.0 → 1.0.1)

### Pre-release Versions

- `0.1.0-alpha.1`: Early alpha release
- `0.1.0-beta.1`: Feature-complete beta
- `0.1.0-rc.1`: Release candidate
- `0.1.0`: Stable release

### Workspace Versioning

All packages in the workspace use the same version number to ensure compatibility.

## Release Types

### Alpha Releases (0.x.x-alpha.x)

- **Purpose**: Early testing and feedback
- **Stability**: APIs may change significantly
- **Documentation**: Basic documentation required
- **Testing**: Core functionality tested

### Beta Releases (0.x.x-beta.x)

- **Purpose**: Feature-complete testing
- **Stability**: API mostly stable, minor changes possible
- **Documentation**: Complete documentation required
- **Testing**: Full test coverage

### Release Candidates (0.x.x-rc.x)

- **Purpose**: Final testing before stable
- **Stability**: Production-ready, only critical fixes
- **Documentation**: Complete and reviewed
- **Testing**: Comprehensive testing including performance

### Stable Releases (1.x.x)

- **Purpose**: Production use
- **Stability**: Breaking changes only in major versions
- **Documentation**: Complete with examples
- **Testing**: Full coverage including real-world scenarios

## Hotfix Process

For critical bugs in released versions:

1. **Create Hotfix Branch**: `hotfix/v1.2.1` from release tag
2. **Apply Fix**: Minimal changes to fix the issue
3. **Test Thoroughly**: Ensure fix doesn't break anything
4. **Manual Release**: Skip normal release process
5. **Update Main**: Cherry-pick or merge fix to main

## Metrics and Monitoring

Track release health:

- **Download Statistics**: Monitor crates.io download numbers
- **Issue Reports**: Track post-release bug reports
- **Documentation**: Monitor docs.rs build status
- **Compatibility**: Track breaking change impact

## Security Releases

For security vulnerabilities:

1. **Private Fix**: Develop fix privately
2. **Coordinate Disclosure**: Follow responsible disclosure
3. **Security Advisory**: Publish GitHub security advisory
4. **Emergency Release**: Fast-track release process
5. **User Notification**: Notify users of critical updates 