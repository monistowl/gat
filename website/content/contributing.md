+++
title = "Contributing"
weight = 100
description = "How to contribute to GAT development"
template = "page.html"
+++

# Contributing to GAT

We welcome contributions from the community! Whether you're fixing bugs, adding features, improving documentation, or providing feedback, your help makes GAT better.

## Ways to Contribute

### 🐛 Report Bugs

Found a bug? [Open an issue]({{ config.extra.repo_url }}/issues/new) with:
- Clear description of the problem
- Steps to reproduce
- Expected vs. actual behavior
- System information (OS, GAT version, Rust version)
- Minimal reproducible example if possible

### ✨ Request Features

Have an idea? [Start a discussion]({{ config.extra.repo_url }}/discussions) or [open an issue]({{ config.extra.repo_url }}/issues/new):
- Describe the feature and use case
- Explain why it would be valuable
- Provide examples of how it would work
- Consider implementation complexity

### 📝 Improve Documentation

Documentation is always a work in progress:
- Fix typos or unclear explanations
- Add examples and use cases
- Improve getting started guides
- Write tutorials or blog posts

Documentation lives in `website/content/docs/` and `docs/` directories.

### 💻 Contribute Code

Ready to code? See the development workflow below.

### 🧪 Test and Provide Feedback

Help us test:
- New features on different platforms
- Edge cases and large systems
- Performance benchmarks
- User experience and ergonomics

## Development Workflow

### 1. Set Up Development Environment

```bash
# Clone the repository
git clone https://github.com/monistowl/gat.git
cd gat

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install system dependencies (Linux)
sudo apt-get install coinor-libcbc-dev

# Install system dependencies (macOS)
brew install coinor-cbc

# Build and test
cargo build
cargo test
```

### 2. Find Something to Work On

- Check [good first issues]({{ config.extra.repo_url }}/labels/good%20first%20issue)
- Look at [help wanted]({{ config.extra.repo_url }}/labels/help%20wanted) issues
- Review the [roadmap]({{ config.extra.repo_url }}/blob/main/docs/ROADMAP.md)
- Propose your own feature or improvement

### 3. Create a Branch

```bash
# Create a feature branch from staging
git checkout staging
git pull origin staging
git checkout -b feature/your-feature-name
```

### 4. Make Your Changes

**Code Style:**
- Run `cargo fmt` before committing
- Run `cargo clippy` and fix warnings
- Write tests for new functionality
- Update documentation as needed

**Commit Messages:**
- Use clear, descriptive commit messages
- Reference issue numbers when applicable
- Follow conventional commits format if possible:
  - `feat: Add new feature`
  - `fix: Fix bug in power flow`
  - `docs: Update contributing guide`
  - `test: Add tests for OPF`

### 5. Test Your Changes

```bash
# Run all tests
cargo test --workspace

# Run specific tests
cargo test -p gat-cli

# Check formatting
cargo fmt --check

# Run clippy
cargo clippy --workspace -- -D warnings

# Build documentation
cargo doc --no-deps --open
```

### 6. Submit a Pull Request

**Before submitting:**
- Ensure all tests pass
- Run `cargo fmt` and `cargo clippy`
- Update documentation if needed
- Add tests for new features
- Rebase on latest staging if needed

**Create PR:**
1. Push your branch: `git push origin feature/your-feature-name`
2. Open a pull request against `staging` (not `main`)
3. Fill out the PR template with:
   - Clear description of changes
   - Related issues
   - Testing performed
   - Breaking changes (if any)

**PR Review Process:**
1. Automated checks run (tests, formatting, clippy)
2. Maintainers review the code
3. You may be asked to make changes
4. Once approved, maintainers merge to staging
5. Changes go to main in the next release

## Code Organization

```
gat/
├── crates/
│   ├── gat-core/       # Core power systems algorithms
│   ├── gat-cli/        # Command-line interface
│   ├── gat-tui/        # Terminal user interface
│   ├── gat-io/         # I/O and data formats
│   ├── gat-algo/       # Algorithms and solvers
│   └── ...
├── docs/               # Original documentation (md files)
├── website/            # Zola website source
│   ├── content/        # Website and docs content
│   └── templates/      # Website templates
├── examples/           # Example datasets and scripts
├── test_data/          # Test datasets
└── scripts/            # Build and release scripts
```

## Development Guidelines

### Code Quality

- **Safety:** Use safe Rust patterns, avoid `unsafe` unless necessary
- **Performance:** Profile before optimizing, but be aware of hot paths
- **Error Handling:** Use `Result` types, provide helpful error messages
- **Testing:** Write unit tests for algorithms, integration tests for CLI
- **Documentation:** Document public APIs with doc comments

### Adding New Features

When adding features:
1. Start with an issue or discussion
2. Get feedback on the approach
3. Write tests first (TDD when possible)
4. Implement incrementally
5. Document in code and user-facing docs
6. Update examples if relevant

### Working with Dependencies

- Prefer well-maintained crates with permissive licenses
- Avoid adding heavy dependencies for small features
- Document why dependencies are needed
- Keep dependencies up to date

## Release Process

Releases follow a **staging → main → tag** workflow:

1. **Develop on staging** - All PRs merge to staging
2. **Run diagnostics** - Comprehensive testing on staging
3. **Build packages** - Create release artifacts
4. **Merge to main** - When ready for release
5. **Tag release** - Manual git tag on main
6. **Deploy** - GitHub Pages, package distribution

See [RELEASE_PROCESS.md]({{ config.extra.repo_url }}/blob/main/RELEASE_PROCESS.md) for details.

## Community Guidelines

### Be Respectful

- Treat all contributors with respect
- Welcome newcomers and help them get started
- Focus on constructive feedback
- Assume good intentions

### Be Clear

- Write clear issue descriptions
- Provide context in PRs
- Ask questions if something is unclear
- Document your thought process

### Be Patient

- Maintainers are volunteers
- Reviews take time
- Not all proposals will be accepted
- Feedback helps everyone learn

## License for Contributions

By contributing to GAT, you agree that your contributions will be licensed under the same license as the project (PolyForm Shield License 1.0.0).

This means:
- Your code can be used in the free version of GAT
- Your code can be used in commercial versions of GAT
- You retain copyright to your contributions
- You grant GAT permission to use your contributions

See [License & Terms](/license/) for details.

## Getting Help

Stuck? Need guidance?

- **Discussions:** [Ask questions]({{ config.extra.repo_url }}/discussions)
- **Issues:** [Report problems]({{ config.extra.repo_url }}/issues)
- **Documentation:** [Read the docs](/docs/)
- **Examples:** [Check examples]({{ config.extra.repo_url }}/tree/main/examples)

## Recognition

Contributors are recognized:
- In commit history
- In release notes
- In the contributor graph
- In special recognition for major contributions

## Quick Reference

```bash
# Start new work
git checkout staging
git pull origin staging
git checkout -b feature/my-feature

# During development
cargo fmt              # Format code
cargo clippy           # Lint code
cargo test             # Run tests
cargo build            # Build project

# Before PR
cargo test --workspace # All tests
cargo fmt --check      # Check formatting
cargo clippy --all     # Check all warnings

# Submit PR
git push origin feature/my-feature
# Open PR on GitHub against 'staging' branch
```

---

**Thank you for contributing to GAT!** Every contribution, no matter how small, helps make power systems analysis better for everyone.
