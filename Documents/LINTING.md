# VIBE Linting & Formatting Guide

This document outlines the linting and formatting setup for the VIBE project, covering both the Rust backend and the CSS/Frontend.

## Tools Summary

| Scope | Tool | Purpose | Configuration |
|-------|------|---------|---------------|
| **Rust** | `clippy` | Static analysis & linting | In-code attributes & `Cargo.toml` |
| **Rust** | `rustfmt` | Code formatting | `src-tauri/.rustfmt.toml` |
| **CSS** | `stylelint` | CSS linting & formatting | `.stylelintrc.json` |
| **General** | `prettier` | General formatting (JSON, TSX) | `.prettierrc.json` |

## Setup

### Prerequisites

1.  **Node.js & npm**: Installed for frontend linting.
2.  **Rust & Cargo**: Installed for backend linting.
3.  **Clippy & Rustfmt**: Ensure these are installed:
    ```bash
    rustup component add clippy rustfmt
    ```

### Installation

The project dependencies include all necessary linting tools. Run:
```bash
npm install
```

## Usage

Commands are integrated into `package.json` for ease of use.

### Linting (Check only)

| Command | Description |
|---------|-------------|
| `npm run lint` | Run all lints (CSS + Rust) |
| `npm run lint:css` | Run CSS linting only |
| `npm run lint:rust` | Run Rust clippy checks |

### Formatting (Fixing issues)

| Command | Description |
|---------|-------------|
| `npm run fmt` | Format all code (CSS + Rust) |
| `npm run fmt:css` | Fix CSS formatting and lint issues |
| `npm run fmt:rust` | Format Rust code using `rustfmt` |

## Integration

### CI/CD (GitHub Actions)
It is recommended to integrate these checks into your CI pipeline. A sample workflow:

```yaml
name: Lint
on: [push, pull_request]
jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
      - run: npm install
      - run: npm run lint:css
      - name: Rust Lint
        run: |
          cd src-tauri
          rustup component add clippy
          cargo clippy -- -D warnings
```

### VS Code Integration
Install the following extensions for a better experience:
- **Rust-analyzer**: For real-time Rust feedback.
- **Stylelint**: For real-time CSS feedback.
- **Prettier**: For automatic formatting on save.

## Formatting Rules

- **Rust**: Uses standard 2021 edition rules with `Windows` line endings and grouped imports.
- **CSS**: Uses `stylelint-config-standard`.
