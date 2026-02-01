# Plugin Development Standards

## Directory Structure

Each plugin follows this structure:

```
plugins/<name>/
├── .claude-plugin/
│   └── plugin.json              # ONLY plugin.json goes here
├── CLAUDE.md                    # Plugin-specific instructions
├── README.md                    # User documentation
├── commands/                    # Slash commands
│   └── <command>.md
├── skills/                      # Agent skills
│   └── <skill>/
│       └── SKILL.md
├── agents/                      # Custom agents
│   └── <agent>.md
├── hooks/
│   └── hooks.json               # Hook configuration
├── scripts/                     # Supporting scripts (minimal)
└── tests/                       # Plugin-specific tests
```

## plugin.json Schema

```json
{
  "name": "plugin-name",
  "version": "1.0.0",
  "description": "Brief description",
  "author": {
    "name": "epiphytic",
    "email": "dev@epiphytic.dev"
  },
  "repository": "https://github.com/epiphytic/gear",
  "license": "MIT",
  "keywords": ["gear", "category"],
  "commands": "./commands/",
  "skills": "./skills/",
  "agents": "./agents/",
  "hooks": "./hooks/hooks.json"
}
```

## Skill Development

SKILL.md frontmatter:
```yaml
---
name: skill-name
description: "When to use this skill (for auto-invocation)"
user-invocable: true
allowed-tools: "Read, Grep, Bash"
---
```

Rules:
- Keep SKILL.md under 500 lines
- Use `allowed-tools` to restrict capabilities
- Move reference material to separate files
- Support both manual and automatic invocation

## Hook Development

Use gear-core for hook logic, not bash scripts:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "${CLAUDE_PLUGIN_ROOT}/../core/target/release/gear-core hook pre-tool-use",
            "timeout": 10
          }
        ]
      }
    ]
  }
}
```

Available hook events:
- `SessionStart` / `SessionEnd` - Session lifecycle
- `UserPromptSubmit` - User submits prompt
- `PreToolUse` - Before tool execution (can block)
- `PostToolUse` / `PostToolUseFailure` - After tool execution
- `Stop` - Session stopping (can prevent)
- `SubagentStart` / `SubagentStop` - Agent lifecycle
- `PreCompact` - Before context compaction
- `Notification` - System notifications

## Command Development

Command frontmatter:
```yaml
---
name: command-name
description: "What this command does"
argument-hint: "[optional-args]"
---
```

## Documentation Standard

All documentation must exist in dual format:
- `.md` - Human-readable markdown
- `.aisp` - Machine-readable AISP format

Generate .aisp using: `npx aisp-converter --input file.md --output file.aisp`

## Version Control

- Use semantic versioning (MAJOR.MINOR.PATCH)
- Angular commit format: `<type>(<scope>): <description>`
- Types: feat, fix, docs, style, refactor, perf, test, build, ci, chore
