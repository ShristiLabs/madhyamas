# Contributing to Madhyamas

Thank you for your interest in contributing to Madhyamas! This document provides guidelines and instructions for contributing to the project.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [How to Contribute](#how-to-contribute)
- [Development Workflow](#development-workflow)
- [Coding Standards](#coding-standards)
- [Testing Guidelines](#testing-guidelines)
- [Documentation](#documentation)
- [Pull Request Process](#pull-request-process)
- [Community](#community)

## Code of Conduct

### Our Pledge

We are committed to providing a welcoming and inclusive environment for all contributors, regardless of experience level, gender, gender identity and expression, sexual orientation, disability, personal appearance, body size, race, ethnicity, age, religion, or nationality.

### Expected Behavior

- Be respectful and considerate
- Use welcoming and inclusive language
- Accept constructive criticism gracefully
- Focus on what's best for the community
- Show empathy towards other community members

### Unacceptable Behavior

- Harassment, trolling, or discriminatory comments
- Personal or political attacks
- Publishing others' private information
- Any conduct that could reasonably be considered inappropriate

## Getting Started

### Prerequisites

- **Rust** 1.75 or later
- **Cargo** (comes with Rust)
- **Node.js** 18+ and npm
- **Git**
- Familiarity with Rust, async programming, and web development

### Setting Up Your Development Environment

1. **Fork the Repository**
   ```bash
   # Click "Fork" on GitHub, then clone your fork
   git clone https://github.com/YOUR_USERNAME/madhyamas.git
   cd madhyamas
   ```

2. **Add Upstream Remote**
   ```bash
   git remote add upstream https://github.com/ShristiLabs/madhyamas.git
   git fetch upstream
   ```

3. **Install Dependencies**
   ```bash
   # Rust dependencies (automatically handled by cargo)
   cargo build
   
   # Frontend dependencies
   cd web
   npm install
   ```

4. **Run Tests**
   ```bash
   cargo test
   ```

## How to Contribute

### Types of Contributions

We welcome various types of contributions:

#### 🐛 Bug Reports
- Use the GitHub issue tracker
- Include steps to reproduce
- Provide system information (OS, Rust version, etc.)
- Include relevant logs or error messages

#### ✨ Feature Requests
- Open an issue to discuss the feature first
- Explain the use case and benefits
- Consider implementation complexity

#### 📝 Documentation
- Improve existing documentation
- Add examples and tutorials
- Fix typos and clarify confusing sections

#### 💻 Code Contributions
- Bug fixes
- New features
- Performance improvements
- Refactoring

#### 🧪 Testing
- Add test coverage
- Improve existing tests
- Report test failures

## Development Workflow

### 1. Create a Branch

```bash
# Update your fork
git checkout main
git pull upstream main

# Create a feature branch
git checkout -b feature/my-new-feature
# or
git checkout -b fix/bug-description
```

### Branch Naming Convention

- `feature/description` - New features
- `fix/description` - Bug fixes
- `docs/description` - Documentation updates
- `refactor/description` - Code refactoring
- `test/description` - Test additions/improvements

### 2. Make Your Changes

Follow the [coding standards](#coding-standards) and write tests for your changes.

### 3. Commit Your Changes

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```bash
# Format: <type>(<scope>): <description>

git commit -m "feat(proxy): add HTTP/2 support"
git commit -m "fix(tls): resolve certificate generation issue"
git commit -m "docs(api): update endpoint documentation"
git commit -m "test(core): add integration tests for session management"
```

**Commit Types:**
- `feat` - New feature
- `fix` - Bug fix
- `docs` - Documentation changes
- `style` - Code style changes (formatting, etc.)
- `refactor` - Code refactoring
- `test` - Adding or updating tests
- `chore` - Maintenance tasks
- `perf` - Performance improvements

### 4. Push to Your Fork

```bash
git push origin feature/my-new-feature
```

### 5. Create a Pull Request

- Go to your fork on GitHub
- Click "New Pull Request"
- Select your branch
- Fill out the PR template
- Link related issues

## Coding Standards

### Rust Code Style

#### Formatting

```bash
# Format all code
cargo fmt --all

# Check formatting
cargo fmt --all -- --check
```

#### Linting

```bash
# Run clippy
cargo clippy --all-targets --all-features

# Treat warnings as errors
cargo clippy --all-targets --all-features -- -D warnings
```

#### Code Quality Rules

**Error Handling**
```rust
// ✅ Good: Use Result and ? operator
pub fn process(&self, data: &[u8]) -> Result<String> {
    let parsed = self.parse(data)?;
    let result = self.transform(parsed)?;
    Ok(result)
}

// ❌ Bad: Using unwrap() in production code
pub fn process(&self, data: &[u8]) -> String {
    self.parse(data).unwrap()
}
```

**Async/Await**
```rust
// ✅ Good: Proper async function
pub async fn fetch_data(&self, url: &str) -> Result<Vec<u8>> {
    let response = reqwest::get(url).await?;
    let bytes = response.bytes().await?;
    Ok(bytes.to_vec())
}
```

**Documentation**
```rust
/// Processes an HTTP request through the proxy
///
/// # Arguments
/// * `request` - The incoming HTTP request
///
/// # Returns
/// * `Ok(Response)` - The processed response
/// * `Err(Error)` - If processing fails
///
/// # Examples
/// ```
/// let response = engine.process_request(&request).await?;
/// ```
pub async fn process_request(&self, request: Request) -> Result<Response> {
    // Implementation
}
```

**Naming Conventions**
- Use `snake_case` for functions and variables
- Use `PascalCase` for types and traits
- Use `SCREAMING_SNAKE_CASE` for constants
- Prefix private items with `_` if unused

**Module Organization**
```rust
// lib.rs
pub mod config;
pub mod proxy;
pub mod tls;

pub use config::ProxyConfig;
pub use proxy::ProxyEngine;

// Error types
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
```

### TypeScript/React Code Style

```typescript
// Use functional components with hooks
export const TrafficList: React.FC<Props> = ({ filter }) => {
  const [traffic, setTraffic] = useState<TrafficEntry[]>([]);
  
  useEffect(() => {
    // Effect logic
  }, [filter]);
  
  return <div>{/* JSX */}</div>;
};

// Use TypeScript types
interface TrafficEntry {
  id: string;
  method: string;
  url: string;
  timestamp: number;
}
```

## Testing Guidelines

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_feature() {
        let result = my_function();
        assert_eq!(result, expected);
    }
    
    #[tokio::test]
    async fn test_async_feature() {
        let result = async_function().await;
        assert!(result.is_ok());
    }
}
```

### Integration Tests

Place integration tests in `tests/` directory:

```rust
// tests/integration_test.rs
use madhyamas_core::ProxyEngine;

#[tokio::test]
async fn test_proxy_flow() {
    let engine = ProxyEngine::new().await.unwrap();
    // Test implementation
}
```

### Test Coverage

- Aim for >80% code coverage
- Test error conditions
- Test edge cases
- Include integration tests for critical paths

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture

# Run tests in specific crate
cargo test -p madhyamas-core
```

## Documentation

### Code Documentation

- Add doc comments to all public items
- Include examples in doc comments
- Document error conditions
- Explain complex algorithms

### User Documentation

When adding features, update:
- `README.md` - If it affects usage
- `docs/API.md` (and the relevant `docs/API_*.md` domain page) - For API changes
- `docs/GETTING_STARTED.md` - For user-facing features
- `docs/ARCHITECTURE.md` - For architectural changes
- `docs/README.md` - Add new docs to the categorized index
- Feature-specific docs in `docs/` (see the index in `docs/README.md`)

The API reference is split by domain: `API_TRAFFIC.md`, `API_INTERCEPT.md`,
`API_SCRIPTS_PLUGINS.md`, `API_CONFIG.md`, `API_ENTERPRISE.md`,
`API_WEBSOCKET_GRPC.md`, with `API.md` as the index. Update the relevant
domain page(s) when adding or changing endpoints.

For new modules in `crates/madhyamas-core/src/`, create a corresponding
`docs/` reference page using the structure in `docs/TEMPLATE.md`.

### Changelog

Update `CHANGELOG.md` with your changes:

```markdown
## [Unreleased]

### Added
- New feature description (#PR_NUMBER)

### Fixed
- Bug fix description (#PR_NUMBER)

### Changed
- Breaking change description (#PR_NUMBER)
```

## Pull Request Process

### Before Submitting

- [ ] Code follows project style guidelines
- [ ] All tests pass locally
- [ ] New tests added for new functionality
- [ ] Documentation updated
- [ ] Commit messages follow conventional commits
- [ ] No merge conflicts with main branch

### PR Template

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Related Issues
Fixes #issue_number

## Testing
Describe testing performed

## Checklist
- [ ] Tests pass
- [ ] Documentation updated
- [ ] Code formatted
- [ ] Clippy warnings resolved
```

### Review Process

1. **Automated Checks**: CI/CD runs tests and linting
2. **Code Review**: Maintainers review your code
3. **Feedback**: Address review comments
4. **Approval**: At least one maintainer approves
5. **Merge**: Maintainer merges your PR

### After Your PR is Merged

- Delete your feature branch
- Update your fork:
  ```bash
  git checkout main
  git pull upstream main
  git push origin main
  ```

## Community

### Communication Channels

- **GitHub Issues**: Bug reports and feature requests
- **GitHub Discussions**: General questions and discussions
- **Discord**: Real-time chat (link in README)
- **Twitter**: [@Madhyamas](https://twitter.com/madhyamas)

### Getting Help

- Check existing documentation
- Search GitHub issues
- Ask in GitHub Discussions
- Join our Discord community

### Recognition

Contributors are recognized in:
- `CONTRIBUTORS.md` file
- Release notes
- Project README

## Development Tips

### Useful Commands

```bash
# Watch for changes and rebuild
cargo watch -x run

# Run clippy with auto-fix
cargo clippy --fix

# Generate documentation
cargo doc --open

# Check dependencies
cargo outdated

# Update dependencies
cargo update
```

### Debugging

```bash
# Enable debug logging
RUST_LOG=debug cargo run

# Use rust-lldb for debugging
rust-lldb target/debug/madhyamas
```

### Performance Profiling

```bash
# Install flamegraph
cargo install flamegraph

# Generate flamegraph
cargo flamegraph
```

## License

By contributing to Madhyamas, you agree that your contributions will be licensed under the same license as the project (MIT OR Apache-2.0).

### Enterprise crate (BSL-1.1)

The `crates/madhyamas-enterprise/` crate is licensed under the Business Source
License 1.1 (BSL-1.1), **not** the MIT OR Apache-2.0 license used by the rest
of the project. See `crates/madhyamas-enterprise/LICENSE-BSL` for the full
text. Contributions to the enterprise crate are accepted under BSL-1.1 and
will eventually convert to the MIT OR Apache-2.0 dual license on the Change
Date (four years from first public distribution of each version).

## Questions?

If you have questions about contributing, feel free to:
- Open a GitHub Discussion
- Ask in our Discord
- Email the maintainers

Thank you for contributing to Madhyamas! 🎉
