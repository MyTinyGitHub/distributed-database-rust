# Homelab Monorepo

A personal monorepo containing a distributed database engine built from scratch in Rust, a language interpreter, and supporting homelab infrastructure.

---

## Projects

### 🗄️ Distributed Database Engine
A distributed database engine built from scratch in Rust. Accepts SQL queries, plans and executes them across distributed partition nodes, and stores data using an LSM tree storage engine.

- Shared-nothing architecture with hash-based partitioning
- Custom SQL subset parser and query planner
- LSM tree storage engine with WAL-first writes
- Streaming join execution

→ See [ARCHITECTURE.md](./ARCHITECTURE.md) for full design documentation

### 🔤 Interpreter
An interpreter for the Monkey language built in Rust, following "Writing an Interpreter in Go". Foundation for the database query language frontend.

→ See [interpreter/README.md](./interpreter/README.md)

---

## Repository Structure

```
monorepo/
├── gateway/              # SQL parser, lexer, CLI interface
├── coordinator/          # Executor, planners, catalog, join module
├── storage-engine/       # Partition node, WAL, MemTable, LSM Tree
├── interpreter/          # Monkey language interpreter in Rust
├── .opencode/
│   └── agents/           # AI code review and architecture agents
├── docs/
│   └── adr/              # Architecture Decision Records
├── ARCHITECTURE.md       # Full system architecture
└── CONTRIBUTING.md       # Development guidelines and rules
```

---

## Getting Started

```bash
# Run all tests
just test

# Check everything (fmt + lint + test)
just check

# Generate and open documentation
just doc

# Sync to public GitHub
just sync
```

---

## Infrastructure

- Self-hosted Gitea as source of truth
- Automatic sync to public GitHub via git subtree
- Netbird VPN for remote access to homelab
- Reverse proxy for external access

---

## Tech Stack

- **Language:** Rust
- **Task runner:** just
- **Version control:** Gitea (self-hosted) + GitHub (public mirror)
- **VPN:** Netbird
- **OS:** Arch Linux
