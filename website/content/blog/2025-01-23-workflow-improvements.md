+++
title = "Streamlined Release Workflow & Website Improvements"
date = 2025-01-23
description = "We've automated GitHub releases, optimized CI workflows, and added comprehensive website content"

[taxonomies]
tags = ["releases", "infrastructure", "documentation"]
categories = ["updates"]

[extra]
author = "GAT Team"
+++

We're excited to share several major improvements to the GAT development workflow and website infrastructure!

## Automated GitHub Releases

When we push a version tag, GAT now automatically:

{% callout(type="success") %}
✅ Builds all 6 package variants (headless/analyst/full × Linux/macOS)
✅ Generates release notes from git history
✅ Creates a GitHub release with all packages attached
✅ Includes installation instructions
{% end %}

**What this means:** Releases are now consistent, reproducible, and require minimal manual work. Just tag and go!

{% code(lang="bash") %}
# Create and push a release tag
git tag -a v0.2.2 -m "Release v0.2.2"
git push origin v0.2.2

# GitHub Actions handles the rest!
{% end %}

## CI Optimization

We've optimized our GitHub Actions workflows to skip unnecessary Rust builds when only documentation or website content changes.

**Impact:** ~95% reduction in CI time for docs/website work!

{% callout(type="tip") %}
**Pro tip:** Website changes now only trigger the deployment workflow, not the full Rust test suite.
{% end %}

## Website Enhancements

The website now includes comprehensive content pages:

- **[About](@/about.md)** - Project background and philosophy
- **[License & Terms](@/license.md)** - Clear licensing information
- **[Contributing](@/contributing.md)** - Development workflow guide
- **[Security](@/security.md)** - Security policies and features

Plus, the site now deploys only from the `main` branch, making it clear which deployment is production.

## What's Next

We're continuing to improve the documentation and examples. Stay tuned for:

- Interactive examples with live results
- Performance benchmark comparisons
- Step-by-step tutorials for common workflows
- Video walkthroughs

## Get Involved

Want to contribute? Check out our [Contributing Guide](@/contributing.md) or open an issue on [GitHub](https://github.com/monistowl/gat/issues).

{% callout(type="info") %}
**Subscribe to updates:** We now have an RSS feed! Add `/gat/rss.xml` to your feed reader.
{% end %}
