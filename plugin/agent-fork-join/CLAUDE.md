# Agent Fork Join

## Activation Conditions

The plugin activates automatically when:

1. The repository has a **GitHub remote** (origin URL contains github.com)
2. You are on the **default branch** (main/master) OR on a **plugin-created branch** (Angular-style: feat/, fix/, etc.)

The plugin will NOT activate for:

- Non-GitHub repositories
- User's own feature branches that don't follow the Angular convention

## Hook Workflow

### UserPromptSubmit (on new prompt)

1. **Checks beads issue status** (if `.beads/current-issue` exists):
   - Syncs with JIRA via beads to get latest status
   - If issue is closed, cleans up tracking
   - Outputs `BEADS_ISSUE_CLOSED=true` signal for Claude to run `/jira:work`
2. Checks if prompt will make code changes (keywords: implement, add, create, fix, etc.)
3. If on default branch AND changes expected:
   - Creates an Angular-style feature branch (feat/, fix/, refactor/, etc.)
   - AI generates branch name based on prompt content
   - Immediately pushes branch to origin
4. **If on existing PR branch with beads issue**:
   - Appends prompt to PR description
   - Comments on beads issue (local) AND directly to JIRA via REST API

### PostToolUse (on file writes)

1. Detects Write/Edit/MultiEdit tool completions
2. **Tracks files** for later commit (does NOT commit immediately)
3. This ensures a single commit per session, not per-file

### Stop (session end)

1. Commits ALL tracked changes in a **single commit**
2. Uses AI to generate Angular-style commit message
3. **If beads issue is tracked** (`.beads/current-issue` exists):
   - Extracts JIRA key from beads issue's `external_ref` field
   - Prepends JIRA ticket ID to commit message (e.g., `PGF-123: feat: add feature`)
   - Enables JIRA Smart Commits
4. Pushes changes to remote
5. Creates a PR (if none exists) with:
   - AI-generated summary
   - Complete original prompt in metadata
   - **JIRA ticket link** (if tracked)
6. **Comments on beads issue** (local) AND directly to JIRA via REST API:
   - For new PRs: PR title, URL, files changed, JIRA ticket
   - For existing PRs: PR URL, files changed in this session
   - **Note**: Beads comments do NOT sync to JIRA, so direct JIRA API is used

## Angular Commit Types

| Type       | Description                                                   |
| ---------- | ------------------------------------------------------------- |
| `build`    | Changes that affect the build system or external dependencies |
| `ci`       | Changes to CI configuration files and scripts                 |
| `docs`     | Documentation only changes                                    |
| `feat`     | A new feature                                                 |
| `fix`      | A bug fix                                                     |
| `perf`     | A code change that improves performance                       |
| `refactor` | A code change that neither fixes a bug nor adds a feature     |
| `test`     | Adding missing tests or correcting existing tests             |

## Commands

### /done - Complete Branch Workflow

Use `/done` to merge your PR and clean up. This runs the `scripts/done-workflow.sh` script which handles:

1. **Check PR status** - Determines if PR is open, merged, or closed
2. **Wait for pending checks** - If GitHub Actions are running, waits with status updates every minute
3. **Merge when ready** - Automatically merges when all checks pass
4. **Handle blockers** - Reports what's blocking and stops if not mergeable:
   - Failing checks → Shows which checks failed, stops
   - Conflicts → Directs user to resolve on GitHub, stops
   - Review required with admin access → **Asks user for confirmation** before using admin override
   - Review required without admin → Stops and requires approval
5. **Comment on beads issue** - "PR merged" (syncs to JIRA if linked)
6. **Run JIRA sync** - Always syncs beads with JIRA after operations
7. **Ask about issue status** - Options: "Done", "In Review", "No change"
   - **Always refer to issues by JIRA key** (e.g., "PGF-123"), not beads ID
8. **Terraform Cloud** - If `/terraform` exists and `TFC_TOKEN` is set:
   - Finds linked non-production workspace
   - Waits for plan to complete
   - Offers to approve apply if user has permission
9. **Switch to main branch**
10. **Pull latest changes**
11. **Delete local feature branch**
12. **Clean up tracking** - Remove `.beads/current-issue` (only if user selects "Done")

```
/done
```

**Environment Variables:**

- `CHECK_INTERVAL`: Seconds between status updates (default: 60)
- `MAX_WAIT_TIME`: Max wait time for checks in seconds (default: 3600)
- `TFC_TOKEN`: Terraform Cloud API token (for TFC integration)

## Beads/JIRA Integration

When `.beads/current-issue` exists (set by `/jira:work`), this plugin automatically:

### On Commit

- Extracts JIRA key from beads issue's `external_ref` field
- Prepends JIRA ticket ID to commit message
- Format: `PGF-123: feat(scope): description`
- Enables JIRA Smart Commits for automatic linking

### On PR Creation

- Includes JIRA ticket in PR title
- Adds JIRA ticket link in PR description
- Comments on beads issue (local) AND directly to JIRA via REST API
  - **Note**: Beads comments do NOT sync to JIRA, so direct JIRA API is used

### On PR Update (new prompt or changes pushed)

- Appends prompt to PR description's "Prompt History" section
- Comments on beads issue (local) AND directly to JIRA via REST API
  - **Note**: Beads comments do NOT sync to JIRA, so direct JIRA API is used

### On /done (when PR merged)

- Syncs with JIRA before outputting beads signals
- Comments "PR merged" on beads issue (local) AND directly to JIRA via API
  - **Note**: Beads comments do NOT sync to JIRA, so direct JIRA API is required
- Asks user about updating issue status (Done, In Review, etc.)
- Runs `bd jira sync --push` after status update (status DOES sync)
- Cleans up `.beads/current-issue` only if user selects "Done"

### JIRA Sync Limitations

| Data Type       | Syncs via `bd jira sync`? | How to Sync                        |
| --------------- | ------------------------- | ---------------------------------- |
| Issue Title     | ✅ Yes                     | `bd jira sync --push`              |
| Issue Status    | ✅ Yes                     | `bd jira sync --push`              |
| Issue Desc      | ✅ Yes                     | `bd jira sync --push`              |
| **Comments**    | ❌ **No**                  | Direct JIRA API (jira-comment.sh)  |

The `beads_add_comment()` function in `hooks/lib/common.sh` automatically handles this by:
1. Adding the comment to beads locally
2. Posting the comment directly to JIRA via REST API (using `plugins/jira/scripts/jira-comment.sh`)

### User-Facing Messages

**Always refer to issues by their JIRA key** (e.g., "PGF-123") in user-facing messages, NOT the beads ID (e.g., "bd-29"). This avoids confusing users who don't know about beads internals.

## Terraform Cloud Integration

When running `/done`, if a `/terraform` directory exists and `TFC_TOKEN` is set, the plugin will:

1. **Find TFC organization** from terraform config (`cloud {}` or `backend "remote"` blocks)
2. **Find linked workspaces** that are connected to the current GitHub repository
3. **Identify non-production workspace** by name patterns:
   - Contains `-dev` (highest priority)
   - Contains `-staging`
   - Contains `-nonprod`
   - Does NOT contain `-prod` or `-production` (fallback)
4. **Check latest run status** on the non-production workspace
5. **Wait for plan** if still in progress (up to 10 minutes)
6. **Report errors** if the plan failed, including error details
7. **Offer to approve** if:
   - Plan succeeded
   - Workspace requires manual approval
   - User has permission to approve

### TFC Scripts

| Script            | Purpose                                    |
| ----------------- | ------------------------------------------ |
| `tfc-check.sh`    | Check workspace status after merge         |
| `tfc-approve.sh`  | Approve a run for apply                    |

### Environment Variables

- `TFC_TOKEN`: Terraform Cloud API token (required for TFC integration)

### Silent Behavior

If `TFC_TOKEN` is not set or `/terraform` doesn't exist, the TFC integration is skipped silently without bothering the user.

## PR Prompt History

Each PR description includes a "Prompt History" section with timestamped collapsible accordions for each prompt submitted during the session. When continuing work on an existing PR branch, new prompts are automatically appended to this history.

## .fork-join Directory

The plugin creates a `.fork-join/` directory to store session state:

```
.fork-join/
├── current_session       # Current session ID
├── tracked_files.txt     # Files changed in this session
└── session-*.json        # Session metadata
```

**Gitignore Handling:**

- When creating `.fork-join/`, the plugin automatically adds it to `.gitignore`
- If `.fork-join/` is already in `.gitignore`, no changes are made
- If user has `!.fork-join` in `.gitignore` (to track session files), the plugin respects that and does NOT re-add the ignore rule

## Multi-Agent Workflow (Future)

The daemon infrastructure supports:

1. Each spawned agent gets its own worktree
2. Agent changes are committed to separate branches
3. FIFO merge queue handles sequential integration
4. Conflict resolution and rebasing when needed
5. Final PR created when all agents complete
