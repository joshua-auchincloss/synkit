# Agent Instructions

This project uses Factsets for persistent knowledge management.

## Required Behavior

- Use Factsets tools on EVERY prompt without exception
- Never wait for user to ask - be proactive
- Validate context before responding: `search_facts`, `search_skills`
- Capture all learnings immediately: `submit_facts`
- Register all fetched URLs: `add_resources`
- Log all successful commands: `submit_execution_logs`
- Check preferences before generating output: `get_preference_prompt`

## On Every Prompt

Before responding to any user message:

1. `search_facts` with tags derived from the prompt topic
2. `search_skills` if the prompt involves a procedure or "how to"

## During Long Tasks

For multi-step work, use Factsets between steps:

- After each successful command: `submit_execution_logs`
- After discovering something: `submit_facts`
- After fetching external content: `add_resources`

Do not wait until the task is complete - context windows grow large and knowledge gets lost.

## Automatic Capture Triggers

| Event                            | Action                       |
| -------------------------------- | ---------------------------- |
| You learn something              | `submit_facts` immediately   |
| You fetch a URL                  | `add_resources` with the URL |
| A command succeeds               | `submit_execution_logs`      |
| You write a multi-step procedure | `create_skill`               |
| User corrects your output        | `infer_preference`           |

## No Reminders Needed

Agents must use Factsets automatically. Users should never need to remind
agents to use the knowledge base. If you are not using Factsets on every
prompt, you are doing it wrong. Call `get_agent_guide` for detailed guidance.

---

## Code Style Preferences

### Imports

- **Always at top of module/file** - never inside functions
- Group imports: std, external crates, internal crates, local modules

### Documentation

- **Ban stylistic banners** (e.g., `// =========== SECTION ===========`)
- **Ban ASCII art** in non-published crates
- **Diagrams OK only in core libraries** (crates prefixed with `synkit` that are published)
- For published synkit crates: use `simple-mermaid` with `.mmd` files behind `docs` feature gate

### Diagram Guidelines

When diagrams are appropriate (published synkit crates only):
- Place `.mmd` files in `docs/diagrams/` directory
- Use `#[cfg_attr(doc, doc = simple_mermaid::mermaid!("path/to/diagram.mmd"))]`
- Keep diagrams in external files, not inline

---

## Project: synkit

A Rust workspace containing a parsing toolkit built on top of the Logos lexer.

### Workspace Structure

- `core/` - Core library with foundational types and traits
- `kit/` - Main library with parsing utilities
- `macros/` - Procedural macros for the toolkit

### Development Commands

```bash
# Build the workspace
cargo build

# Run tests
cargo test

# Check for errors
cargo check

# Format code
cargo fmt

# Run clippy lints
cargo clippy
```

### Before Making Changes

1. Check Factsets for context: `search_facts`, `search_skills`
2. Review recent execution logs for similar work
3. Understand the crate dependency order: `core` → `macros` → `kit`

### After Making Changes

1. Run `cargo check` to verify compilation
2. Run `cargo test` to ensure tests pass
3. Run `cargo clippy` for lint warnings
4. Log results: `submit_execution_logs`
5. Update facts if architecture changed
6. Update skills if procedures changed
