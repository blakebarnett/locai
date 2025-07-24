# Contributing to Locai

Thank you for your interest in contributing to Locai! This guide will help you get started.

## 🚀 Release Process

This project uses automated releases with [release-please](https://github.com/googleapis/release-please) and conventional commits. Here's how it works:

### Conventional Commits

We follow the [Conventional Commits](https://www.conventionalcommits.org/) specification. Your commit messages should be structured as follows:

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

#### Types

- `feat`: A new feature
- `fix`: A bug fix
- `docs`: Documentation only changes
- `style`: Changes that do not affect the meaning of the code (white-space, formatting, missing semi-colons, etc)
- `refactor`: A code change that neither fixes a bug nor adds a feature
- `perf`: A code change that improves performance
- `test`: Adding missing tests or correcting existing tests
- `build`: Changes that affect the build system or external dependencies
- `ci`: Changes to our CI configuration files and scripts
- `chore`: Other changes that don't modify src or test files
- `revert`: Reverts a previous commit

#### Examples

```bash
# Feature addition
git commit -m "feat: add semantic search capabilities"

# Bug fix
git commit -m "fix: resolve memory leak in entity extraction"

# Documentation update
git commit -m "docs: update API examples in README"

# Breaking change
git commit -m "feat!: redesign memory storage API

BREAKING CHANGE: Memory.store() now returns a Result instead of panicking"
```

### Release Workflow

1. **Development**: Make changes using conventional commits
2. **PR Review**: Create pull requests with conventional commit messages
3. **Automatic Release PR**: release-please creates a PR with version bumps and changelog
4. **Release**: When the release PR is merged, packages are automatically published to crates.io

### Version Bumping

release-please automatically determines version bumps based on commit types:

- `fix`: patch version bump (0.1.0 → 0.1.1)
- `feat`: minor version bump (0.1.0 → 0.2.0)  
- `BREAKING CHANGE` or `!`: major version bump (0.1.0 → 1.0.0)

## 🛠️ Development Setup

### Prerequisites

- Rust 1.85.0+ (supports Rust 2024 edition)
- Git

### Getting Started

1. **Clone the repository**
   ```bash
   git clone https://github.com/blakebarnett/locai.git
cd locai
   ```

2. **Install dependencies and build**
   ```bash
   cargo build --all
   ```

3. **Run tests**
   ```bash
   cargo test --all
   ```

4. **Run examples**
   ```bash
   cargo run --example getting_started
   ```

### Code Quality

We maintain high code quality standards:

- **Tests**: All new features should include tests
- **Documentation**: Public APIs should be documented
- **Formatting**: Run `cargo fmt` before committing
- **Linting**: Run `cargo clippy` and fix warnings
- **Examples**: Update examples if you change public APIs

### CI Pipeline

Our CI runs on every PR:
- ✅ **Tests**: Full test suite across all features
- ✅ **Format**: Code formatting checks
- ✅ **Clippy**: Linting and code quality  
- ✅ **Examples**: Verify examples compile

## 📦 Publishing Process

### Automatic Publishing

When releases are created, packages are automatically published to crates.io in dependency order:

1. `locai` (core library)
2. `locai-server` (depends on locai)
3. `locai-cli` (depends on locai)

### Manual Publishing (if needed)

If automatic publishing fails, you can manually publish:

```bash
# Publish core library first
cd locai && cargo publish

# Wait for crates.io to process, then publish dependent crates
cd ../locai-server && cargo publish
cd ../locai-cli && cargo publish
```

## 🎯 Contribution Areas

We welcome contributions in these areas:

- **🔍 Entity Extraction**: New domain-specific extractors
- **🚀 Performance**: Optimization and benchmarking
- **📚 Documentation**: Examples, guides, and API docs
- **🧪 Testing**: Test coverage and integration tests
- **🌐 Integrations**: New storage backends or ML models
- **🐛 Bug Fixes**: Issues and edge cases

## 📞 Getting Help

- **Issues**: [GitHub Issues](https://github.com/blakebarnett/locai/issues)
- **Discussions**: [GitHub Discussions](https://github.com/blakebarnett/locai/discussions)
- **Documentation**: [docs.rs/locai](https://docs.rs/locai)

## 📄 License

By contributing to Locai, you agree that your contributions will be licensed under the MIT License. 