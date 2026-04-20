---
tags:
  - multiplexing
  - workspace
---
# `switch_to_non_empty_workspace_on_empty = true`

When the currently-active workspace becomes empty (all its windows,
tabs, and panes have been closed), wezterm by default picks the first
non-empty workspace it finds and promotes it into the active GUI
view.

Set this to `false` to disable that behavior. The active workspace is
left empty, and the GUI window closes normally if nothing else is
keeping it open. Other workspaces remain exactly where they are —
they are not silently promoted into view.

This is useful when you use a hidden workspace (e.g. `"parked"`) as a
stash for backgrounded panes and don't want it surfacing just because
the visible workspace emptied out via `exit`, `Ctrl-D`, or similar.

```lua
config.switch_to_non_empty_workspace_on_empty = false
```

The default is `true` (original behavior).
