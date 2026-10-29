# Sayanox Core

**Minimal browser engine** written in Rust for **Sayanox Browser**.

> Built by [Sayan Mahata](https://github.com/sayan9168) (Sayan The Researcher)  
> Part of the [Sayanox](https://github.com/sayan9168/sayanox) ecosystem

---

## What is Sayanox Core?

Sayanox Core is a from-scratch, educational + experimental browser engine.  
It aims to implement the essential pipeline of a modern rendering engine:

```
HTML → DOM → CSS → Style Cascade → Layout → Paint → Display
```

**Not** a full Chromium/Blink replacement.  
It is intentionally minimal, clean, and written in pure Rust so that one person (with AI agents) can understand and extend every part.

### Target Features (Roadmap)

| Stage | Component              | Status     |
|-------|------------------------|------------|
| 1     | HTML Tokenizer + Parser | Done      |
| 2     | DOM Tree               | Done      |
| 3     | CSS Parser             | Done      |
| 4     | Style Cascade          | Done      |
| 5     | Block Layout Engine    | Done      |
| 6     | Software Rasterizer    | Done      |
| 7     | Simple Window (winit / softbuffer) | Done |
| 8     | Basic JavaScript (later) | Future  |

---

## Project Structure

```text
sayanox-core/
├── Cargo.toml
├── README.md
└── src/
    ├── main.rs
    ├── html/          # Tokenizer + Parser → DOM
    ├── css/           # CSS Parser
    ├── style/         # Cascade & Computed Style
    ├── layout/        # Box model + Block layout
    ├── paint/         # Rasterization
    └── util/          # Shared helpers
```

---

## Building

```bash
git clone https://github.com/sayan9168/sayanox-core.git
cd sayanox-core
cargo test
cargo run                # Opens a window and renders the example page
cargo run -- --headless  # Runs the full pipeline without opening a window
```

---

## Philosophy

- **Readable first** — every module should be understandable by a single developer
- **No magic** — avoid heavy frameworks; prefer clear data structures
- **Incremental** — each stage produces visible output before moving to the next
- **Sayanox spirit** — original work, self-contained where possible

---

## Related Projects

- [Sayanox Language](https://github.com/sayan9168/sayanox) — Original systems programming language
- Sayanox Browser (coming soon) — The browser that will use this engine

---

**License:** Sayanox License / MIT (to be decided)  
**Author:** Sayan Mahata ([@sayan9168](https://github.com/sayan9168))
