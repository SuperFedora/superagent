# SuperAgent

[![Release](https://img.shields.io/github/v/release/SuperFedora/superagent?label=SuperAgent)](https://github.com/SuperFedora/superagent/releases)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)

Standalone autonomous 16-bit workstation agent (`superagent`) for OpenClaw, local/LAN Ollama, coding harnesses, and Cast Comms notifications on **[SuperFedora](https://superfedora.github.io/)**.

```bash
superagent                        # Launch 16-bit Autonomous Agent & Ollama TUI Studio
superagent status                 # Show active harness, Ollama VRAM models & daemon state
superagent daemon [--once]        # Run autonomous workstation watchdog & Cast Comms relay
superagent notify <role> <msg>    # Relay message through 16-bit Cast Comms (supernotify)
superagent set-ollama <ip:port>   # Connect local or LAN Ollama server and sync Pi models
superagent ask "<prompt>"         # Query active AI agent or Ollama model from CLI
```

* **Cast Comms Integration:** Relays autonomous status, alerts, and completions via `supernotify` (`AGY // OPENCLAW` portrait).

**Full documentation & AI harness guide: [superfedora.github.io](https://superfedora.github.io/)**

Licensed under the Apache License, Version 2.0.
