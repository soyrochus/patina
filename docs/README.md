# Patina Documentation

Welcome to the **Patina** documentation set.  
Patina is a native Rust desktop client for interacting with AI model providers and Model Context Protocol (MCP) services.

This directory contains high-level documentation for developers and contributors.

---

## Document Index

| Document | Description |
|-----------|--------------|
| [architecture.md](architecture.md) | Full technical overview of Patina’s crates, runtime design, and current improvement areas. |
| [coding_guidelines.md](coding_guidelines.md) | Coding, style, and architecture rules for all contributors and AI co-generation tools. |

---

## Contribution Workflow

1. Read both documents before introducing new modules or dependencies.  
2. Keep all new code aligned with the crate boundaries described in **architecture.md**.  
3. Use **coding_guidelines.md** as the prompt basis for co-generative tools (e.g. Copilot, ChatGPT).  
4. Submit pull requests referencing the guideline sections you followed or modified.  
5. Keep documentation up to date — code and docs should evolve together.

---

## 🛠 Recommended Reading

- [egui Book](https://docs.rs/egui/latest/egui/) – Immediate mode UI patterns  
- [tracing crate](https://docs.rs/tracing/) – Structured logging and telemetry  
- [thiserror crate](https://docs.rs/thiserror/) – Idiomatic error handling  
- [tokio](https://tokio.rs/) – Async runtime used in Patina core  

---

_Last updated: {{ date("2025-11-11") }}_
