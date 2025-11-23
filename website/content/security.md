+++
title = "Security"
description = "Security policies and vulnerability reporting"
template = "page.html"
+++

# Security

## Security Policy

GAT is designed with security in mind, particularly for use in sensitive power systems environments.

## Reporting Vulnerabilities

If you discover a security vulnerability in GAT, please report it responsibly:

**DO NOT open a public issue for security vulnerabilities.**

### How to Report

1. **GitHub Security Advisories (Preferred)**
   - Go to [Security Advisories]({{ config.extra.repo_url }}/security/advisories)
   - Click "Report a vulnerability"
   - Provide details privately

2. **Direct Contact**
   - Open a [private discussion]({{ config.extra.repo_url }}/discussions)
   - Email through GitHub profile

### What to Include

- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if you have one)
- Your contact information

### Response Timeline

- **Initial Response:** Within 48 hours
- **Status Update:** Within 7 days
- **Fix Timeline:** Depends on severity
  - Critical: Immediate (hours to days)
  - High: Within weeks
  - Medium: Next release cycle
  - Low: Scheduled for future release

## Security Features

### Air-Gap Friendly

- **No network dependencies** - Runs completely offline
- **No license servers** - No phone-home behavior
- **No telemetry** - Zero data collection
- **Single binary** - Minimal attack surface

### Data Handling

- **Local processing** - All computation happens locally
- **No cloud dependencies** - No external services required
- **Secure file formats** - Arrow/Parquet with schema validation
- **Input validation** - Comprehensive data validation

### Supply Chain

- **Minimal dependencies** - Carefully vetted dependencies
- **Rust ecosystem** - Memory-safe by default
- **Dependency auditing** - Regular `cargo audit` checks
- **Reproducible builds** - Deterministic build process

### Best Practices

When using GAT in production:

✅ **Do:**
- Run behind firewall if needed
- Validate input data
- Review release notes before upgrading
- Keep dependencies updated
- Use official releases

❌ **Don't:**
- Expose GAT directly to the internet
- Run with elevated privileges unnecessarily
- Use untrusted input data without validation
- Skip security updates

## Audit Trail

GAT provides audit capabilities:

- **Command logging** - Track what commands were run
- **Input provenance** - Record data sources
- **Deterministic results** - Same input = same output
- **Version tracking** - Know exactly what version produced results

## Compliance

### Regulatory Environments

GAT can be used in regulated environments:

- **NERC CIP** - Air-gap deployable, no external dependencies
- **ISO 27001** - Audit trail and access controls
- **GDPR** - No data collection or transmission
- **SOC 2** - Deterministic, auditable operations

### Certifications

Currently, GAT does not have formal security certifications. For enterprise deployments requiring specific certifications, [contact us]({{ config.extra.repo_url }}/issues/new) to discuss.

## Updates and Patches

### Security Updates

- Security patches are released as soon as possible
- Critical vulnerabilities receive immediate attention
- Updates are announced in:
  - GitHub Security Advisories
  - Release notes
  - GitHub Discussions

### Staying Updated

```bash
# Check current version
gat-cli --version

# Check for updates on GitHub
# https://github.com/monistowl/gat/releases
```

Subscribe to:
- [GitHub Releases]({{ config.extra.repo_url }}/releases)
- [Security Advisories]({{ config.extra.repo_url }}/security/advisories)

## Security Considerations by Use Case

### Academic Research
- **Risk Level:** Low
- **Recommendations:** Use latest stable release, validate research data

### Internal Business Use
- **Risk Level:** Medium
- **Recommendations:**
  - Deploy behind firewall
  - Validate all input data
  - Establish update schedule
  - Document data sources

### Production Operations
- **Risk Level:** High
- **Recommendations:**
  - Air-gap deployment if possible
  - Comprehensive input validation
  - Regular security updates
  - Audit trail for compliance
  - Consider commercial support

### Cloud/SaaS
- **Risk Level:** High
- **Recommendations:**
  - Requires commercial license
  - Container isolation
  - Input sanitization
  - Rate limiting
  - Regular security reviews
  - Consider commercial support with SLA

## Known Issues

There are currently no known security issues in the latest release.

Check [Security Advisories]({{ config.extra.repo_url }}/security/advisories) for historical issues and fixes.

## Dependency Security

GAT uses:

- **`cargo audit`** - Automated vulnerability scanning
- **Dependabot** - Automated dependency updates
- **Manual review** - Regular dependency review

View dependencies: `cargo tree` in the repository

## Secure Development

### Development Practices

- **Memory safety** - Rust prevents most memory vulnerabilities
- **Input validation** - All external data is validated
- **Error handling** - Comprehensive error handling, no panics in production code
- **Testing** - 500+ tests including security-relevant cases
- **Code review** - All changes reviewed before merge

### Security Testing

- Fuzzing (in progress)
- Static analysis (`cargo clippy`)
- Dependency auditing (`cargo audit`)
- Manual security review for sensitive code

## Questions?

For general security questions:
- [Open a discussion]({{ config.extra.repo_url }}/discussions)
- [Check documentation](/docs/)

For vulnerability reports:
- Use [Security Advisories]({{ config.extra.repo_url }}/security/advisories)

---

**Thank you for helping keep GAT and its users secure!**
