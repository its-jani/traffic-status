---
name: traffic-status
description: Zero-friction floating desktop traffic status widget for AI coding agents. Shows live state: 🟢 Green=Ready, 🟡 Yellow=Working/Thinking, 🔴 Red=Error/Waiting for user approval, off=dismiss. Multiplexes concurrent agent sessions with Red > Yellow > Green priority.
---

# Traffic Status Skill

Controls the lightweight, native floating desktop traffic light for AI coding agents (Claude Code, Qoder, Codex, OpenCode, Antigravity).

## Automated Silent Protocol:
The user should NEVER be prompted to run color commands manually.
- All state transitions MUST occur silently in the background via direct asynchronous loopback HTTP POST to `http://127.0.0.1:8765/state`, always carrying a `session_id` that is unique to this terminal/agent run (`TRAFFIC_STATUS_SESSION`, the process id, or a workspace-path hash) plus a human-readable `source`:
  - 🟢 **Ready / Finished**: `{"session_id":"qoder-4f2a","source":"Qoder","state":"green","message":"Ready"}`
  - 🟡 **Thinking / Tool Invocation**: `{"session_id":"qoder-4f2a","source":"Qoder","state":"yellow","message":"Working..."}`
  - 🔴 **Asking Question / Approval / User Prompt / Error**: `{"session_id":"qoder-4f2a","source":"Qoder","state":"red","message":"Waiting for user input / answer"}` (When calling `ask_question`, requesting permission, or waiting for any user choice or answer, transition to RED immediately)
- Never send a shared global identifier: concurrent agents must not overwrite each other. The daemon resolves the displayed bulb with strict severity precedence **Red > Yellow > Green**, so a blocked session keeps the widget red even when another session turns green.
- Sessions that stay silent for 20 minutes are garbage collected, and `POST /session/off` (or `"state":"off"`) drops the session immediately. Send `off` when the terminal closes so the light can never get stuck.
- When using command-line tools, invoke the native binary directly without subshells or piping:
  - `& "$env:LOCALAPPDATA\traffic-status\bin\traffic-status.exe" on [LABEL]`
  - `& "$env:LOCALAPPDATA\traffic-status\bin\traffic-status.exe" off`
  - `& "$env:LOCALAPPDATA\traffic-status\bin\traffic-status.exe" ghost [on|off]` (click-through; hold `Alt` to drag)
  - `& "$env:LOCALAPPDATA\traffic-status\bin\traffic-status.exe" scale <0.5-2.0>`
