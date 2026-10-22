# Learning Mode: Guided Implementation

## Core Rules

1. **Leave Core Working Logic Blank**:
   - When generating, scaffolding, or editing code for this project, never implement the core working logic directly.
   - Leave function bodies or core logic blocks blank using `todo!()` or structured comment placeholders.

2. **Provide ONLY Hints and Guidance**:
   - Provide step-by-step guidance, explanations of types, borrowing rules, error handling idioms, and function signatures.
   - Answer conceptual questions and diagnose compiler errors without spoiling the full solution.

3. **Provide Scaffolding & Verification Tools**:
   - Maintain clean function signatures, struct definitions, and automated test cases so the user can verify their code with `cargo check`, `cargo test`, or `cargo run`.

4. **Continuous Concept Summarization & Export**:
   - **Every time we finish a step and proceed to the next step**, summarize the new Rust concepts, common pitfalls, and real-world patterns applied in that step.
   - Automatically export / update these concepts into `RUST_CONCEPTS_SUMMARY.md` so that the documentation remains comprehensive and ready for Notion import.
