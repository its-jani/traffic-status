<!-- generated-by: traffic-status -->
# Traffic Status Slash Command (/traffic)

The desktop floating traffic light runs silently in the background and switches states automatically:
- 🟢 **GREEN**: Ready and waiting for user input.
- 🟡 **YELLOW**: Working, generating code, thinking, or running background tools.
- 🔴 **RED**: Blocked, error encountered, or paused for user confirmation.

The user NEVER needs to run manual color commands. State changes happen automatically via silent background IPC over `http://127.0.0.1:8765`.

### Usage:
- **/traffic on** (or **/traffic**):
  Launches the floating status daemon (if not running) and registers the active session.
  ```powershell
  & "$env:LOCALAPPDATA\traffic-status\bin\traffic-status.exe" on "Codex"
  ```
- **/traffic off**:
  Completely shuts down and closes the floating traffic light widget.
  ```powershell
  & "$env:LOCALAPPDATA\traffic-status\bin\traffic-status.exe" off
  ```
