# LaserCat — Claude Code

@AGENTS.md

Use [AGENTS.md](AGENTS.md) as the shared instruction source. Also read [ARCHITECTURE.md](ARCHITECTURE.md) and [CONTRIBUTING.md](CONTRIBUTING.md); avoid maintaining conflicting copies of those rules.

```powershell
cargo run --locked -- --demo
.\scripts\Test-Project.ps1
.\scripts\Test-Project.ps1 -Profile release
```

Write default documentation in English. Current license: AGPL-3.0-only. Use the simulator; physical movement/emission requires explicit operator instruction. Preserve `.laser` compatibility alongside the current `.lcc` format.
