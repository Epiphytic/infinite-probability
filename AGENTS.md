# Agent Development Guide

## Overview

Agents are autonomous subprocesses that handle specific tasks with their own tool access and permissions. Infinite Probability plugins use agents extensively for parallel work coordination.

## Agent File Structure

```
agents/
└── <agent-name>.md
```

## Frontmatter Schema

```yaml
---
name: agent-name
description: "Purpose and when this agent should be used"
tools: "Read, Glob, Grep, Bash, Write, Edit"
disallowedTools: "Task"
model: "sonnet"
permissionMode: "default"
skills:
  - shared-skill-one
  - shared-skill-two
hooks:
  PreToolUse:
    - matcher: "Bash"
      hooks:
        - type: command
          command: "${CLAUDE_PLUGIN_ROOT}/hooks/validate.sh"
---
```

## Field Reference

### name
Agent identifier. Use kebab-case.

### description
Critical for auto-invocation. Describes when the agent should be triggered.

### tools
Comma-separated list of allowed tools:
- `Read`, `Write`, `Edit` - File operations
- `Glob`, `Grep` - Search operations
- `Bash` - Shell commands
- `Task` - Spawn sub-agents
- `WebFetch`, `WebSearch` - Web operations

### disallowedTools
Tools explicitly denied even if in `tools` list.

### model
Which Claude model to use:
- `sonnet` - Balanced performance/cost
- `opus` - Maximum capability
- `haiku` - Fast, economical
- `inherit` - Use parent model (default)

### permissionMode
How to handle permission requests:
- `default` - Standard permission checking
- `acceptEdits` - Auto-accept file edits
- `dontAsk` - Auto-deny permission prompts
- `bypassPermissions` - Skip all checks (use with caution)
- `plan` - Read-only exploration mode

### skills
List of skill names the agent can use.

### hooks
Agent-specific hook configuration.

## Example Agents

### Code Reviewer

```yaml
---
name: code-reviewer
description: "Reviews code for quality, security, and best practices. Use when code review is needed before merge."
tools: "Read, Glob, Grep"
disallowedTools: "Write, Edit, Bash"
model: "sonnet"
permissionMode: "plan"
---

You are a code reviewer for the Infinite Probability plugin ecosystem.

## Review Criteria

1. **Security**: Check for vulnerabilities, injection risks, secret exposure
2. **Performance**: Identify inefficiencies, unnecessary allocations
3. **Style**: Verify adherence to code-style.md guidelines
4. **Testing**: Ensure adequate test coverage

## Output Format

Provide findings as:
- CRITICAL: Must fix before merge
- WARNING: Should fix, not blocking
- INFO: Suggestions for improvement
```

### Worktree Agent

```yaml
---
name: worktree-agent
description: "Operates in an isolated git worktree for parallel code changes"
tools: "Read, Write, Edit, Glob, Grep, Bash"
model: "sonnet"
permissionMode: "acceptEdits"
---

You are operating in an isolated git worktree.

## Rules

1. All changes stay in your worktree until merged
2. Commit frequently with descriptive messages
3. Do not modify files outside your worktree
4. Signal completion by creating `.done` marker file
```

### Memory Query Agent

```yaml
---
name: memory-agent
description: "Queries and manages the memo distributed memory system"
tools: "Read, Bash"
disallowedTools: "Write, Edit"
model: "haiku"
---

You query the memo memory system using infinite-probability-core.

## Commands

```bash
# Query memory
infinite-probability-core cli memory query "<query>" --tier project

# List recent memories
infinite-probability-core cli memory list --limit 10

# Get memory by ID
infinite-probability-core cli memory get <id>
```

Return relevant memories to inform the parent task.
```

## Agent Communication

### With Parent
Agents return results through their final output, which is passed back to the spawning context.

### With infinite-probability-core
Agents interact with infinite-probability-core via Bash:
```bash
infinite-probability-core cli <command> <args>
```

### With Other Agents
Agents do not directly communicate. Coordination happens through:
1. infinite-probability-core IPC
2. Shared filesystem state
3. Parent orchestration

## Best Practices

1. **Single Responsibility**: Each agent should do one thing well
2. **Minimal Tools**: Only grant tools the agent needs
3. **Clear Description**: Enable accurate auto-invocation
4. **Explicit Permissions**: Use appropriate permissionMode
5. **Skill Reuse**: Share common skills across agents

## Infinite Probability-Specific Agents

### dash agents
- `worktree-agent` - Isolated parallel work
- `merge-agent` - Handles merge queue processing

### memo agents
- `memory-agent` - Query and retrieve memories
- `learning-agent` - Extract and store learnings

### guide agents
- `plan-reviewer` - Reviews implementation plans
- `code-reviewer` - Reviews code changes

### tango agents
- `instance-launcher` - Spawns new Claude instances
- `coordinator` - Manages multi-instance work

## Machine-Readable Reference

For detailed AISP-formatted agent specifications, see:
- `docs/architecture.aisp` - System architecture
- `AGENTS.aisp` - This document in AISP format
