# AI Agent Log Access - Usage Guide

## Log File Location

```
~/.fontatlas/logs/app.log
```

On macOS, this expands to:
```
/Users/YOUR_USERNAME/.fontatlas/logs/app.log
```

## Log Format

JSON Lines (one JSON object per line):

```json
{"timestamp":"2025-12-19T18:00:00Z","level":"info","source":"frontend","message":"Fetched 150 fonts"}
{"timestamp":"2025-12-19T18:00:01Z","level":"error","source":"backend","message":"Parse failed","details":{"path":"/font.ttf"}}
```

## AI Agent Usage

### Real-time Monitoring

```bash
# Tail logs in real-time
tail -f ~/.fontatlas/logs/app.log

# Follow with colored output (if you have grc)
tail -f ~/.fontatlas/logs/app.log | jq -C
```

### Search & Filter

```bash
# Find all errors
grep "error" ~/.fontatlas/logs/app.log

# Find specific message
grep "Failed to parse" ~/.fontatlas/logs/app.log

# Count errors
grep -c "error" ~/.fontatlas/logs/app.log
```

### Parse with jq

```bash
# Get only error logs
cat ~/.fontatlas/logs/app.log | jq 'select(.level=="error")'

# Get messages from last 10 logs
tail -10 ~/.fontatlas/logs/app.log | jq '.message'

# Get logs with details
cat ~/.fontatlas/logs/app.log | jq 'select(.details != null)'

# Extract specific field from details
cat ~/.fontatlas/logs/app.log | jq 'select(.details.path) | .details.path'
```

### Python Script Example

```python
import json
from pathlib import Path

log_file = Path.home() / ".fontatlas" / "logs" / "app.log"

# Read all logs
with open(log_file) as f:
    logs = [json.loads(line) for line in f]

# Filter errors
errors = [log for log in logs if log['level'] == 'error']

# Analyze
print(f"Total logs: {len(logs)}")
print(f"Errors: {len(errors)}")

# Get recent errors
recent_errors = errors[-5:]
for error in recent_errors:
    print(f"{error['timestamp']}: {error['message']}")
```

## Frontend Logger

The frontend logger automatically intercepts all console output:

```typescript
// Automatically logged
console.log("Font loaded:", fontData);
console.error("Parse failed:", error);
console.warn("Deprecated API");

// Manual logging with details
import { logger } from './utils/logger';

await logger.log('info', 'Custom event', {
  fontId: '123',
  action: 'preview'
});
```

## Disable Logging

```typescript
import { logger } from './utils/logger';

// Disable
logger.setEnabled(false);

// Re-enable
logger.setEnabled(true);
```

## Log Rotation

Currently, logs append indefinitely. For production, consider:

```bash
# Manual rotation
mv ~/.fontatlas/logs/app.log ~/.fontatlas/logs/app.log.old
# App will create new file on next log

# Or truncate
> ~/.fontatlas/logs/app.log
```

Future: Implement automatic rotation in Rust logger.

---

**Created**: 2025-12-19  
**Author**: AI Agent (Gemini)
