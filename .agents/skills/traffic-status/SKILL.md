---
name: traffic-status
description: Controls the desktop floating traffic light status widget for AI coding agents (Claude Code, Qoder, Codex, OpenCode, Antigravity). Use to show status light (green=ready, yellow=working/thinking, red=error/confirmation needed, off=dismiss).
---

# Traffic Status Skill

Controls the lightweight, native floating desktop traffic light for AI coding agents.

## Usage Commands:
- `/traffic on` or `traffic-status on [LABEL]`: Launch the background daemon (if not running) and activate the status light in 🟢 **Green** (Ready).
- `/traffic yellow [MSG]` or `traffic-status yellow [MSG]`: Set status to 🟡 **Yellow** while generating code, thinking, or running tools.
- `/traffic green [MSG]` or `traffic-status green [MSG]`: Set status to 🟢 **Green** when ready for input or when task is completed.
- `/traffic red [MSG]` or `traffic-status red [MSG]`: Set status to 🔴 **Red** when blocked, encountering errors, or requiring user confirmation.
- `/traffic off` or `traffic-status off`: Completely removes and terminates the floating traffic light widget.

