# GAT Website

This directory contains the source for the GAT website, built with [Zola](https://www.getzola.org/).

## Structure

```
website/
├── config.toml          # Zola configuration
├── content/             # Markdown content
│   └── _index.md       # Homepage content
├── templates/           # HTML templates
│   └── index.html      # Homepage template
├── static/              # Static files (CSS, JS, images)
│   └── main.js         # Interactive JavaScript
└── public/              # Build output (generated, not committed)
```

## Local Development

To build and serve the website locally:

```bash
# Install Zola (if not already installed)
# On macOS:
brew install zola

# On Linux:
wget https://github.com/getzola/zola/releases/download/v0.18.0/zola-v0.18.0-x86_64-unknown-linux-gnu.tar.gz
tar xzf zola-v0.18.0-x86_64-unknown-linux-gnu.tar.gz
sudo mv zola /usr/local/bin/

# Build and serve
cd website
zola serve

# Or just build
zola build
```

The site will be available at `http://127.0.0.1:1111`.

## Deployment

The website is automatically deployed to GitHub Pages when changes are pushed to `main` or `staging`.

- **Workflow:** `.github/workflows/deploy-website.yml`
- **Deployment target:** `gh-pages` branch
- **Live URL:** https://monistowl.github.io/gat

The workflow:
1. Checks out the repository
2. Installs Zola
3. Builds the site from the `website/` directory
4. Deploys the `website/public/` directory to the `gh-pages` branch

## Content Updates

- To update the homepage content, edit `content/_index.md`
- To update the homepage template, edit `templates/index.html`
- To update configuration (title, description, URLs), edit `config.toml`
- To add static assets (images, additional CSS/JS), add them to `static/`

## Links

The website includes links to:
- Main repository: https://github.com/monistowl/gat
- Documentation: `/docs` directory in the repo
- README: Main repo README
- AGENTS.md: Agent integration guide
- RELEASE_PROCESS.md: Contributing and release guide
- Examples: `/examples` directory
- Issues and discussions

All links are configured in `config.toml` under `[extra]` and used in templates via `{{ config.extra.repo_url }}`.
# Force rebuild
