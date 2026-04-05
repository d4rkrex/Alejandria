# Claude Desktop Integration for Alejandria

Complete MCP configuration examples for integrating Alejandria persistent memory into Claude Desktop.

## Quick Start

1. **Build Alejandria** (if you haven't already):

```bash
cd /path/to/alejandria
cargo build --release --all-features
```

2. **Install binary to a permanent location**:

```bash
# Linux/macOS - Install to user bin
mkdir -p ~/.local/bin
cp target/release/alejandria ~/.local/bin/

# Verify installation
~/.local/bin/alejandria --version
```

3. **Locate your Claude Desktop configuration file**:

```bash
# macOS
~/Library/Application Support/Claude/claude_desktop_config.json

# Windows
%APPDATA%\Claude\claude_desktop_config.json

# Linux
~/.config/Claude/claude_desktop_config.json
```

4. **Copy the example configuration** and customize:

```bash
# Create config if it doesn't exist
touch ~/Library/Application\ Support/Claude/claude_desktop_config.json

# Copy example (macOS)
cp claude_desktop_config.json.example \
   ~/Library/Application\ Support/Claude/claude_desktop_config.json

# IMPORTANT: Edit the file and replace /home/yourusername with your actual home directory
```

5. **Restart Claude Desktop** completely (quit and relaunch)

6. **Verify integration**:

In Claude Desktop, try:
```
You: Can you check my memory system health?
Claude: [Uses mem_health tool] Your Alejandria memory system is healthy...

You: Store this memory: "Learned about Alejandria MCP integration"
Claude: [Uses mem_store tool] I've stored that memory...
```

---

## Configuration Examples

### 1. Minimal Configuration (Recommended for Quick Start)

```json
{
  "mcpServers": {
    "alejandria": {
      "command": "/home/yourusername/.local/bin/alejandria",
      "args": ["serve"],
      "env": {
        "ALEJANDRIA_DB_PATH": "/home/yourusername/.local/share/alejandria/claude.db"
      }
    }
  }
}
```

**Replace**:
- `/home/yourusername` → Your actual home directory
  - macOS: `/Users/yourname`
  - Linux: `/home/yourname`
  - Windows: `C:/Users/YourName` (use forward slashes or `C:\\Users\\YourName`)

---

### 2. Recommended Configuration (With Logging)

```json
{
  "mcpServers": {
    "alejandria": {
      "command": "/home/yourusername/.local/bin/alejandria",
      "args": ["serve"],
      "env": {
        "ALEJANDRIA_DB_PATH": "/home/yourusername/.local/share/alejandria/claude.db",
        "RUST_LOG": "info",
        "RUST_BACKTRACE": "1"
      }
    }
  }
}
```

**Logging levels**:
- `error` - Only errors (production)
- `warn` - Warnings and errors (recommended)
- `info` - General information (development)
- `debug` - Detailed debug info (troubleshooting)
- `trace` - Very verbose (debugging only)

**Check logs**:
```bash
# macOS
tail -f ~/Library/Logs/Claude/mcp*.log

# Linux
tail -f ~/.config/Claude/logs/mcp*.log
```

---

### 3. Advanced Configuration (Tuned Search & Decay)

```json
{
  "mcpServers": {
    "alejandria": {
      "command": "/home/yourusername/.local/bin/alejandria",
      "args": ["serve"],
      "env": {
        "ALEJANDRIA_DB_PATH": "/home/yourusername/.local/share/alejandria/claude.db",
        "ALEJANDRIA_SEARCH_LIMIT": "20",
        "ALEJANDRIA_SEARCH_MIN_SCORE": "0.3",
        "ALEJANDRIA_SEARCH_BM25_WEIGHT": "0.3",
        "ALEJANDRIA_SEARCH_COSINE_WEIGHT": "0.7",
        "ALEJANDRIA_DECAY_AUTO_DECAY": "true",
        "ALEJANDRIA_DECAY_PRUNE_THRESHOLD": "0.1",
        "RUST_LOG": "warn"
      }
    }
  }
}
```

**Environment variable reference**:

| Variable | Default | Description |
|----------|---------|-------------|
| `ALEJANDRIA_DB_PATH` | `~/.local/share/alejandria/alejandria.db` | Database file path |
| `ALEJANDRIA_SEARCH_LIMIT` | `10` | Maximum search results returned |
| `ALEJANDRIA_SEARCH_MIN_SCORE` | `0.3` | Minimum relevance score (0.0-1.0) |
| `ALEJANDRIA_SEARCH_BM25_WEIGHT` | `0.3` | Keyword search weight (hybrid mode) |
| `ALEJANDRIA_SEARCH_COSINE_WEIGHT` | `0.7` | Vector similarity weight (hybrid mode) |
| `ALEJANDRIA_DECAY_AUTO_DECAY` | `true` | Auto-decay memories before search |
| `ALEJANDRIA_DECAY_PRUNE_THRESHOLD` | `0.1` | Prune memories with weight < threshold |

---

### 4. Multiple Databases (Personal vs Work)

```json
{
  "mcpServers": {
    "alejandria-personal": {
      "command": "/home/yourusername/.local/bin/alejandria",
      "args": ["serve"],
      "env": {
        "ALEJANDRIA_DB_PATH": "/home/yourusername/.local/share/alejandria/personal.db",
        "RUST_LOG": "warn"
      }
    },
    "alejandria-work": {
      "command": "/home/yourusername/.local/bin/alejandria",
      "args": ["serve"],
      "env": {
        "ALEJANDRIA_DB_PATH": "/home/yourusername/.local/share/alejandria/work.db",
        "RUST_LOG": "warn"
      }
    }
  }
}
```

Claude will see both MCP servers and you can specify which one to use:
```
You: Store this in my work memory: "Completed Q1 security audit"
Claude: [Uses alejandria-work] Stored in work memory...
```

---

### 5. Windows Configuration

```json
{
  "mcpServers": {
    "alejandria": {
      "command": "C:/Users/YourName/.local/bin/alejandria.exe",
      "args": ["serve"],
      "env": {
        "ALEJANDRIA_DB_PATH": "C:/Users/YourName/AppData/Local/alejandria/claude.db",
        "RUST_LOG": "info"
      }
    }
  }
}
```

**Notes**:
- Use forward slashes (`/`) or escaped backslashes (`\\`)
- Include `.exe` extension for the binary
- Paths are case-insensitive on Windows

---

### 6. Custom Installation Path

If you installed Alejandria to a custom location:

```json
{
  "mcpServers": {
    "alejandria": {
      "command": "/opt/alejandria/bin/alejandria",
      "args": ["serve"],
      "env": {
        "ALEJANDRIA_DB_PATH": "/var/lib/alejandria/memories.db",
        "RUST_LOG": "info"
      }
    }
  }
}
```

---

## Troubleshooting

### Issue: Claude says "Tool not found"

**Cause**: MCP server failed to start

**Debug**:
1. Test binary path manually:
   ```bash
   /home/yourusername/.local/bin/alejandria --version
   ```

2. Check Claude logs:
   ```bash
   # macOS
   tail -f ~/Library/Logs/Claude/mcp*.log
   
   # Look for error messages like:
   # "Command not found: /home/yourusername/.local/bin/alejandria"
   ```

3. Common fixes:
   - Use **absolute paths** (not `~` or relative paths)
   - Verify binary exists: `ls -la /path/to/alejandria`
   - Check execute permissions: `chmod +x /path/to/alejandria`

---

### Issue: Database permission errors

**Error in logs**: "Permission denied" when accessing database file

**Fix**:
```bash
# Create database directory with correct permissions
mkdir -p ~/.local/share/alejandria
chmod 755 ~/.local/share/alejandria

# If database file exists, fix permissions
chmod 644 ~/.local/share/alejandria/claude.db
```

---

### Issue: Config changes not taking effect

**Fix**:
1. Save `claude_desktop_config.json`
2. **Completely quit** Claude Desktop (not just close window)
   - macOS: `Cmd+Q` or right-click dock icon → Quit
   - Windows: Right-click system tray → Exit
3. Relaunch Claude Desktop
4. Wait 5-10 seconds for MCP servers to initialize

---

### Issue: "sqlite-vec not available" warning

**Impact**: Vector similarity search disabled, BM25 keyword search still works.

**Options**:
1. **Accept it**: Keyword search (BM25/FTS5) is often sufficient for most use cases
2. **Rebuild without embeddings** (smaller binary):
   ```bash
   cargo build --release --no-default-features
   cp target/release/alejandria ~/.local/bin/
   ```
3. **Report issue** if you built with `--all-features` and still see the warning

---

### Issue: Claude responds slowly after asking about memories

**Cause**: Large database or slow search

**Fixes**:
1. **Prune old memories**:
   ```bash
   alejandria decay --dry-run  # See what would be pruned
   alejandria decay            # Actually prune
   ```

2. **Reduce search limit**:
   ```json
   "env": {
     "ALEJANDRIA_SEARCH_LIMIT": "10"  # Reduce from 20
   }
   ```

3. **Increase min score** (more selective):
   ```json
   "env": {
     "ALEJANDRIA_SEARCH_MIN_SCORE": "0.5"  # Increase from 0.3
   }
   ```

---

### Issue: Memory tools showing in wrong context

**Symptom**: Claude tries to use memory tools when you don't want it to

**Fix**: Temporarily disable Alejandria server:
```json
{
  "mcpServers": {
    "_alejandria_disabled": {
      "command": "/home/yourusername/.local/bin/alejandria",
      "args": ["serve"]
    }
  }
}
```

Rename key to `alejandria` to re-enable.

---

## Available MCP Tools

Alejandria exposes **20 MCP tools** to Claude:

### Memory Tools (11 tools)
- `mem_store` - Store new memory or update via topic_key
- `mem_recall` - Search memories with hybrid BM25+vector search
- `mem_update` - Update existing memory by ID
- `mem_forget` - Soft-delete a memory
- `mem_list_topics` - List all topics with counts
- `mem_stats` - Get system statistics
- `mem_health` - Check system health
- `mem_consolidate` - Consolidate topic into summary
- `mem_embed_all` - Generate embeddings for all memories
- `mem_decay` - Manually trigger temporal decay
- `mem_prune` - Prune low-weight memories

### Memoir Tools (9 tools)
- `memoir_create` - Create knowledge graph container
- `memoir_list` - List all memoirs
- `memoir_show` - Show memoir details with graph
- `memoir_add_concept` - Add concept to memoir
- `memoir_refine` - Update concept definition
- `memoir_search` - Search concepts within memoir
- `memoir_search_all` - Search concepts across all memoirs
- `memoir_link` - Link concepts with typed relation
- `memoir_inspect` - Inspect concept neighborhood

See `docs/AGENT_INSTRUCTIONS.md` for complete tool reference with examples.

---

## Example Usage Patterns

### Pattern 1: Storing Important Information

```
You: Remember this: I prefer Rust over Python for systems programming because of memory safety and performance.

Claude: [Uses mem_store with topic_key="preferences/programming-languages"]
I've stored that preference. I'll remember your preference for Rust in systems programming contexts.
```

### Pattern 2: Recalling Past Conversations

```
You: What did we discuss about authentication last week?

Claude: [Uses mem_recall with query="authentication"]
Based on my memory, we discussed:
- JWT token implementation with refresh tokens
- Session security best practices
- OAuth2 integration considerations
```

### Pattern 3: Building Knowledge Graphs

```
You: Create a memoir called "rust-patterns" for tracking Rust design patterns we discuss.

Claude: [Uses memoir_create]
I've created the "rust-patterns" memoir. I can now track concepts and relationships as we discuss them.

You: Add the Builder pattern as a concept.

Claude: [Uses memoir_add_concept]
Added "Builder Pattern" to rust-patterns. As we discuss related patterns, I'll link them together.
```

### Pattern 4: Topic Organization

```
You: Show me what topics I have memories about.

Claude: [Uses mem_list_topics]
You have memories organized in these topics:
- development (45 memories)
- security (23 memories)
- learning (12 memories)
- preferences (8 memories)
```

---

## Best Practices

1. **Use topic_keys for persistent facts**:
   ```
   You: Remember my database password is in 1Password under "prod-db"
   [Claude stores with topic_key="credentials/prod-db"]
   ```

2. **Use topics for organization**:
   ```
   You: Store this in my "learning" topic: Learned about MCP protocol today
   ```

3. **Consolidate periodically**:
   ```
   You: Consolidate my "learning" topic to create a summary
   [Claude uses mem_consolidate to create high-level summary]
   ```

4. **Check health regularly**:
   ```
   You: Check my memory system health
   [Claude uses mem_health to verify database, FTS, embeddings]
   ```

5. **Prune old memories**:
   ```
   You: Clean up my old low-relevance memories
   [Claude uses mem_decay and mem_prune]
   ```

---

## Security Considerations

1. **Database location**: Store in user-local directory (not shared/cloud-synced)
2. **Sensitive data**: Be mindful of what you ask Claude to remember
3. **Database backup**: Regularly backup your database file:
   ```bash
   cp ~/.local/share/alejandria/claude.db \
      ~/.local/share/alejandria/backups/claude-$(date +%Y%m%d).db
   ```
4. **Multiple contexts**: Use separate databases for personal vs. work via multiple MCP servers

---

## Performance Tips

1. **Set min_score appropriately**:
   - `0.3` - Broad recall (more results, some less relevant)
   - `0.5` - Balanced (recommended)
   - `0.7` - Strict (fewer but highly relevant results)

2. **Limit search results**:
   - `10` - Fast, focused (recommended)
   - `20` - More comprehensive
   - `50+` - May slow down responses

3. **Enable auto-decay**:
   ```json
   "ALEJANDRIA_DECAY_AUTO_DECAY": "true"
   ```
   This keeps your database lean and search fast.

4. **Consolidate topics periodically**:
   - Reduces memory count
   - Creates high-level summaries
   - Improves search relevance

---

## Advanced: Custom Search Tuning

Hybrid search blends two ranking methods:

1. **BM25** (keyword matching) - Good for exact terms, technical names
2. **Cosine similarity** (semantic vectors) - Good for conceptual matches

Default weights: 30% BM25 + 70% cosine

**Tune for your use case**:

```json
{
  "env": {
    "ALEJANDRIA_SEARCH_BM25_WEIGHT": "0.5",     // Increase keyword importance
    "ALEJANDRIA_SEARCH_COSINE_WEIGHT": "0.5"   // Decrease semantic importance
  }
}
```

**When to adjust**:
- **More keyword-focused** (technical docs, code, exact terms): `0.5 BM25 / 0.5 cosine`
- **More semantic-focused** (concepts, ideas, natural language): `0.2 BM25 / 0.8 cosine`
- **Balanced** (default): `0.3 BM25 / 0.7 cosine`

---

## Support

- **Full documentation**: [docs/AGENT_INSTRUCTIONS.md](../../docs/AGENT_INSTRUCTIONS.md)
- **MCP tool reference**: [tools/README.md](../../tools/README.md)
- **Deployment guide**: [docs/DEPLOYMENT.md](../../docs/DEPLOYMENT.md)
- **Issues**: https://github.com/yourusername/alejandria/issues
