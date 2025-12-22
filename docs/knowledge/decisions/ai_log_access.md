# AI Agent Log Access

## Context

Cần enable AI agent (Antigravity) để đọc được logs từ FontAtlas app để:
- Debug issues without manual inspection
- Automated testing and validation
- Monitor app behavior during development
- Capture errors and warnings for analysis

## Decision

**File-based logging approach** thay vì REST API:

### Why File-based?
- ✓ Simpler implementation
- ✓ No network overhead
- ✓ Works offline
- ✓ AI agent can read files directly
- ✓ Persistent logs for historical analysis

### Implementation Strategy

1. **Frontend logs** → Write to file via Tauri command
2. **Backend logs** → Rust logging to file
3. **Structured format** → JSON lines for easy parsing
4. **Log rotation** → Prevent file size issues

## Architecture

```
┌─────────────────┐
│  React App      │
│  console.log()  │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Tauri Command   │
│ write_log()     │
└────────┬────────┘
         │
         ▼
┌─────────────────────────────────┐
│ Log File                        │
│ ~/.fontatlas/logs/app.log       │
│ (JSON Lines format)             │
└─────────────────────────────────┘
         │
         ▼
┌─────────────────┐
│ AI Agent        │
│ (Antigravity)   │
│ reads & parses  │
└─────────────────┘
```

## Log Format

**JSON Lines** (one JSON object per line):

```json
{"timestamp":"2025-12-19T18:00:00Z","level":"info","source":"frontend","message":"Fetched 150 fonts"}
{"timestamp":"2025-12-19T18:00:01Z","level":"error","source":"backend","message":"Failed to parse font","details":{"path":"/path/to/font.ttf","error":"Invalid TTF"}}
```

## Benefits

1. **AI can tail logs**: `tail -f ~/.fontatlas/logs/app.log`
2. **Grep/search**: `grep "error" app.log`
3. **Parse with jq**: `cat app.log | jq 'select(.level=="error")'`
4. **Historical analysis**: Keep logs for debugging past issues

## Related

- [AI Console Port](../task.md#ai-console-port-for-automated-testing)
- [Testing Strategy](../testing/automated_testing.md)

---

**Created**: 2025-12-19  
**Author**: AI Agent (Gemini)
