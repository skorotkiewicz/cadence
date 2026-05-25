# Cadence

Small marker tracking for source files.

Cadence turns comments like this:

```rust
// $$todo handle empty input
```

into stable, checkable items:

```rust
// $$todo:1:open handle empty input
```

and mirrors them to Markdown in `.cadence/todo.md`.

## Install

```sh
cargo install --path .
```

Or run it from this checkout:

```sh
cargo run -- <command>
```

## Quick Start

```sh
cadence init
```

Add markers to any source file:

```rust
// $$todo handle empty input
// $$fixme avoid duplicate work
// $$hack remove temporary branch
```

The prefix comes from `.cadence/config.yml`.

Stage files and commit them to Cadence:

```sh
cadence add src/main.rs
cadence commit
```

Cadence assigns IDs in the source file and writes Markdown checklists:

```md
- [ ] $$todo:1:open - handle empty input
```

Check an item in `.cadence/*.md`, then run:

```sh
cadence commit
```

The source marker status changes from `open` to `done`.

Add notes below any Markdown item; Cadence keeps them with that item:

```md
- [x] $$todo:3:done - open final flux
  add
  support
  for
  multiline notes
```

## Commands

```sh
cadence init           # create .cadence/
cadence add <path>     # stage a source file
cadence commit         # sync source markers and Markdown
cadence reset          # clear staged files
```

## Files

```text
.cadence/
  config.yml           # marker prefix
  schemas.yml          # default marker types
  db.json              # tracked items
  staged.json          # staged files
  <type>.md            # generated checklist
```
