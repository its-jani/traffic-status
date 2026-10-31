---
name: traffic-status
description: Zero-friction floating desktop traffic status widget for AI coding agents. Shows live state: 🟢 Green=Ready, 🟡 Yellow=Working/Thinking, 🔴 Red=Error/Waiting for user approval, off=dismiss.
---

# Traffic Status Skill

Controls the lightweight, native floating desktop traffic light for AI coding agents (Claude Code, Qoder, Codex, OpenCode, Antigravity).

## Automated Silent Protocol:
The user should NEVER be prompted to run color commands manually.
- All state transitions MUST occur silently in the background via direct asynchronous loopback HTTP POST to `http://127.0.0.1:8765/state`:
  - 🟢 **Ready / Finished**: `{"session_id":"cli-session","state":"green","message":"Ready"}`
  - 🟡 **Thinking / Tool Invocation**: `{"session_id":"cli-session","state":"yellow","message":"Working..."}`
  - 🔴 **Approval / User Prompt / Error**: `{"session_id":"cli-session","state":"red","message":"Waiting for approval"}`
- When using command-line tools, invoke the native binary directly without subshells or piping:
  - `& "$env:LOCALAPPDATA\traffic-status\bin\traffic-status.exe" on [LABEL]`
  - `& "$env:LOCALAPPDATA\traffic-status\bin\traffic-status.exe" off`
