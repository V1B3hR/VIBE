# 🤝 Contributing to VIBE DAW

First off, thank you for considering contributing to **VIBE DAW**!

## 📜 Licensing & CLA
VIBE DAW uses a **Dual-Licensing Model (GPLv3 / Commercial License)**.

Before submitting any Pull Request:
1. Review the [LICENSE](LICENSE) file.
2. Read and accept the [Contributor License Agreement (CLA)](CLA.md).
3. By opening a Pull Request on [https://github.com/V1B3hR/VIBE](https://github.com/V1B3hR/VIBE), you agree to the terms of the CLA.

## 🛠️ Local Development Setup

### Requirements
- [Rust](https://www.rust-lang.org/tools/install) (Edition 2021)
- [Node.js](https://nodejs.org/) (v18+)

### Commands
```bash
# Install frontend dependencies
npm install

# Run Vite + Tauri dev environment
npm run tauri dev

# Run all test suites
npm run test:all
```

## 🧪 Code Quality Standards
- All Rust code must pass `cargo clippy` and `cargo test`.
- All TypeScript/React code must pass `npm run test:ui` and `npm run lint`.
