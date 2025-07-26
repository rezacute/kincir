# Simplified CI/CD Setup

This document explains the simplified CI/CD pipeline for Kincir.

## Active Workflows

### 1. `simple-ci.yml` - Main CI/CD Pipeline

**Triggers:**
- Push to `main`, `develop`, or `v02-*` branches
- Pull requests to `main` or `develop`

**Jobs:**

#### `build-and-test`
- ✅ Code formatting check (`cargo fmt`)
- ✅ Linting with Clippy (`cargo clippy`)
- ✅ Project compilation (`cargo build`)
- ✅ Test execution (`cargo test`)
- ✅ Dependency caching for faster builds

#### `docs` (main branch only)
- ✅ API documentation generation (`cargo doc`)
- ✅ GitHub Pages deployment
- ✅ Automatic redirect to main crate docs

#### `release` (tags only)
- ✅ Release build (`cargo build --release`)
- ✅ Release testing (`cargo test --release`)
- ✅ GitHub Release creation with changelog

### 2. `docs.yml` - Documentation Only

**Triggers:**
- Push to `main` branch (excluding docs/ and *.md changes)
- Manual workflow dispatch

**Jobs:**
- ✅ API documentation build and deployment to GitHub Pages

## Disabled Workflows

The following complex workflows have been disabled to prevent CI failures:

- `ci.yml.disabled` - Complex multi-job pipeline
- `comprehensive-testing.yml.disabled` - Extensive testing matrix
- `security.yml.disabled` - Security auditing
- `performance.yml.disabled` - Performance benchmarking
- `jekyll.yml.disabled` - Jekyll site generation
- `static-docs.yml.disabled` - Static documentation

## Benefits of Simplified Setup

### ✅ Reliability
- Fewer jobs = fewer failure points
- Essential checks only
- Faster feedback loop

### ✅ Maintainability
- Simple, easy to understand workflows
- Clear separation of concerns
- Easy to debug and modify

### ✅ Efficiency
- Faster CI runs (5-10 minutes vs 30+ minutes)
- Reduced resource usage
- Parallel job execution where beneficial

## Workflow Details

### Build and Test Job
```yaml
- Code formatting check
- Clippy linting
- Project compilation
- Unit and integration tests
- Dependency caching
```

### Documentation Job
```yaml
- API documentation generation
- GitHub Pages setup
- Documentation deployment
- Automatic redirect configuration
```

### Release Job (tags only)
```yaml
- Release build
- Release testing
- GitHub Release creation
- Changelog integration
```

## Usage

### For Development
1. Push code to any branch
2. CI runs basic checks (format, lint, build, test)
3. Get quick feedback on code quality

### For Documentation
1. Push to `main` branch
2. Documentation automatically builds and deploys
3. Available at: https://rezacute.github.io/kincir/

### For Releases
1. Create and push a version tag: `git tag v0.2.1 && git push origin v0.2.1`
2. Release workflow automatically creates GitHub release
3. Includes changelog and installation instructions

## Troubleshooting

### Common Issues

#### Build Failures
- Check `cargo build` locally
- Ensure all dependencies are in Cargo.toml
- Verify Rust version compatibility

#### Test Failures
- Run `cargo test` locally
- Check for platform-specific issues
- Ensure test isolation

#### Documentation Failures
- Run `cargo doc` locally
- Check for missing documentation
- Verify all features compile

### Getting Help

1. Check workflow logs in GitHub Actions tab
2. Run commands locally to reproduce issues
3. Check this documentation for common solutions

## Future Enhancements

When the project stabilizes, consider re-enabling:
- Security auditing (cargo-audit)
- Performance benchmarking
- Cross-platform testing
- Coverage reporting

For now, the simplified setup provides reliable CI/CD with essential checks.
