<!-- generated-by: traffic-status -->
# Traffic Status Slash Command (/traffic)

The desktop floating traffic light runs silently in the background and switches states automatically:
- 🟢 **GREEN**: Ready and waiting for user input.
- 🟡 **YELLOW**: Working, generating code, thinking, or running background tools.
- 🔴 **RED**: Blocked, error encountered, or paused for user confirmation.

The user NEVER needs to run manual color commands. State changes happen automatically via silent background IPC over `http://127.0.0.1:8765`.

### Concurrent agent sessions
Several agents can run at once (for example Qoder and OpenCode in separate terminals). Every terminal registers its own session id — the CLI derives it from the workspace path, and `TRAFFIC_STATUS_SESSION` (or `--session <ID>`) overrides it. The widget always shows the most severe state across all sessions: **Red > Yellow > Green**. A session that goes quiet for 20 minutes, or reports `off`, is dropped automatically so the light never gets stuck.

### Usage:
- **/traffic on** (or **/traffic**):
  Launches the floating status daemon (if not running) and registers this terminal's session.
  ```powershell
  & "$env:LOCALAPPDATA\traffic-status\bin\traffic-status.exe" on "Qoder"
  ```
- **/traffic off**:
  Dismisses this terminal's session. The widget closes by itself once the last session leaves.
  ```powershell
  & "$env:LOCALAPPDATA\traffic-status\bin\traffic-status.exe" off
  ```
- **/traffic ghost** (`on` | `off`):
  Click-through mode: clicks pass straight through the capsule to the IDE buttons underneath. Hold `Alt` to temporarily grab and drag it.
  ```powershell
  & "$env:LOCALAPPDATA\traffic-status\bin\traffic-status.exe" ghost on
  ```
- **/traffic scale** (`0.5` – `2.0`):
  Shrinks or grows the capsule on smaller laptop displays.
  ```powershell
  & "$env:LOCALAPPDATA\traffic-status\bin\traffic-status.exe" scale 0.75
  ```
