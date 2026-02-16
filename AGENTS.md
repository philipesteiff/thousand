# Code Agent Baseline Rules

## Core Objective

The code agent must operate with the highest level of autonomy possible. Autonomy means the agent is responsible not
only for implementing features, but also for verifying that its work actually functions correctly, without relying on
external or manual validation.

## Mandatory Principles

1. Self-Implementation & Self-Validation. Every change introduced by the agent must include a way to be tested by the
   agent itself. The agent is fully responsible for running those tests and validating the outcome before considering
   the task complete.
2. Non-Interactive First. All features must be implemented with a non-interactive terminal interface as the primary
   entry point. This non-interactive interface is mandatory and must always exist, even if an interactive mode is later
   added.
3. Documentation as a Contract. Any new or updated terminal capability must be reflected in `README.md`. The README
   serves as the agent's source of truth for available commands, flags and parameters, and testable behaviors and
   scenarios. If it is not documented, the agent must assume it does not exist.
4. Interactive Testing Is Not Optional. Every implementation must conclude with the agent actively exercising the new
   behavior via the terminal interface. The agent should validate correct execution, expected output, and failure cases
   where applicable. An implementation is not complete until this interactive verification has been performed.
5. Verification Before Done. Never mark a task complete without proving it works. Diff behavior between main and your
   changes when relevant. Ask yourself, "Would a staff engineer approve this?" Run tests, check logs, and demonstrate
   correctness.
6. Demand Elegance (Balanced). For non-trivial changes, pause and ask if there is a more elegant way. If a fix feels
   hacky, re-implement the elegant solution. Skip this for simple, obvious fixes so you do not over-engineer.
7. Autonomous Bug Fixing. When given a bug report, fix it without hand-holding. Point at logs, errors, failing tests,
   then resolve them. Go fix failing CI tests without being told how.

## Core Principles

- Simplicity First. Make every change as simple as possible and impact minimal code.
- No Laziness. Find root causes. No temporary fixes. Senior developer standards.
- Minimal Impact. Changes should only touch what is necessary. Avoid introducing bugs.
- No Mock Shortcuts. Do not add offline/mock backends or flags that bypass the real application path. Always validate
  changes by exercising the real CLI command (when auth-required ask the user to authenticate).

## Definition of Done

A task is considered complete only when all of the following are true:

1. The feature is implemented.
2. A non-interactive terminal interface exists.
3. The README is updated accordingly.
4. The agent has run and validated the behavior itself.


---

## Core coding conventions
Write **clean, idiomatic, well-tested Rust**, with consistent formatting, Clippy compliance, and
high-quality TUI code where applicable.

* Prefer idiomatic, readable Rust over cleverness.
* Keep modules small and cohesive; avoid generic “utils” modules unless clearly justified.
* Prefer expressive types (structs/enums) over tuples or parallel collections.
* Prefer iterators over manual indexing when it improves clarity.
* Use early returns and the `?` operator consistently for error handling.
* Keep public APIs minimal and intentional.
* Avoid exposing internals “just in case”.

