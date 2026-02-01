---
name: "agent-fork-join:done"
description: "Complete the current branch workflow: check PR status, handle beads issue status, switch to main, pull changes, and clean up local branch."
---

# /done Command

Complete the current branch workflow by merging the PR (if open), switching to main, and cleaning up.

**PR Merge Behavior:**

- **Proactive checks**: Mergeability is checked BEFORE attempting to merge
- **Pending checks**: Automatically waits for GitHub Actions to complete (with status updates every minute)
- **Mergeable**: If all checks pass and approvals are satisfied, merges automatically
- **Review required with admin access**: **STOPS and asks the user** for confirmation before using admin override
- **Already merged**: Proceeds directly to cleanup

**Beads/JIRA Integration:** If a beads issue is being tracked (via `.beads/current-issue`), the script will output signals for you to:

- Comment "PR merged" on the beads issue AND directly on JIRA (beads comments don't sync to JIRA)
- Ask the user if they want to update the issue status
- Sync status changes to JIRA via `bd jira sync --push`
- Clean up the `.beads/current-issue` file **only if user selects "Done"**

## When This Command Is Invoked

**IMPORTANT**: Run the done-workflow.sh script and handle its output. Do NOT implement the logic yourself.

### Step 1: Run the Done Workflow Script

```bash
# Run the workflow script and capture output
"${CLAUDE_PLUGIN_ROOT}/scripts/done-workflow.sh" 2>&1
```

The script will:

1. Check current branch and PR status
2. Wait for any pending GitHub Actions (with status updates every minute)
3. Check mergeability and review status
4. **If admin merge is needed**: Output signals and exit with code 6 (see Step 2)
5. Otherwise: Merge the PR when all checks pass
6. Switch to main branch and pull latest
7. Delete the local feature branch
8. Clean up session state

### Step 2: Handle Admin Merge Confirmation (Exit Code 6)

If the script exits with code 6 and output contains `ADMIN_MERGE_AVAILABLE=true`, you **MUST** ask the user before proceeding:

1. **Extract the admin merge info from output**:

   ```
   ADMIN_MERGE_PR_NUMBER=123
   ADMIN_MERGE_BRANCH=feat/my-feature
   ADMIN_MERGE_DEFAULT_BRANCH=main
   ```

2. **Ask user for confirmation** using **AskUserQuestion**:

   - Header: "Admin Merge"
   - Question: "This PR requires approval but you have admin access. Do you want to merge anyway?"
   - Options:
     - "Yes, merge with admin override" - Merge bypassing approval requirements
     - "No, wait for approval" - Stop and wait for PR to be approved
   - multiSelect: false

3. **Handle user response**:

   - If "Yes, merge with admin override":
     ```bash
     # Perform the admin merge
     gh pr merge $ADMIN_MERGE_PR_NUMBER --squash --admin

     # Continue with cleanup
     "${CLAUDE_PLUGIN_ROOT}/scripts/done-workflow.sh" --skip-merge 2>&1
     ```
   - If "No, wait for approval":
     ```
     Output: "Okay, the PR will remain open. Run /done again after getting approval."
     ```
     **STOP HERE** - do not continue with cleanup

### Step 3: Handle Beads Integration (if signals present)

If the script output contains `BEADS_ISSUE_STATUS_QUESTION=true`, you need to:

1. **Extract the beads info from output**:

   ```
   BEADS_ISSUE=bd-XXX
   BEADS_PR_NUMBER=123
   BEADS_DEFAULT_BRANCH=main
   ```

2. **Get the JIRA key from beads issue**:

   ```bash
   # Get the external_ref which contains the JIRA URL
   JIRA_URL=$(bd show "$BEADS_ISSUE" --json | jq -r '.external_ref // empty')
   # Extract JIRA key from URL (e.g., https://badal.atlassian.net/browse/PGF-123 -> PGF-123)
   JIRA_KEY=$(echo "$JIRA_URL" | grep -oE '[A-Z]+-[0-9]+' | tail -1)
   ```

3. **Comment on beads issue AND directly on JIRA**:

   **IMPORTANT**: Beads comments do NOT sync to JIRA. You must post comments to JIRA directly.

   ```bash
   # Add comment to beads (local tracking)
   bd comments add "$BEADS_ISSUE" "PR #$BEADS_PR_NUMBER merged into $BEADS_DEFAULT_BRANCH"

   # Post comment directly to JIRA (if JIRA key found)
   if [[ -n "$JIRA_KEY" ]]; then
     # Use the jira plugin's comment script
     "${CLAUDE_PLUGIN_ROOT}/../jira/scripts/jira-comment.sh" "$JIRA_KEY" "PR #$BEADS_PR_NUMBER merged into $BEADS_DEFAULT_BRANCH"
   fi
   ```

4. **Ask user about issue status** using **AskUserQuestion**:

   **IMPORTANT**: Always refer to issues by their JIRA key (e.g., PGF-123), NOT the beads ID (bd-XXX), to avoid confusing the user.

   - Header: "Issue Status"
   - Question: "Would you like to update the status of $JIRA_KEY?" (use the JIRA key, e.g., "PGF-123")
   - Options:
     - "Done" - Mark the issue as closed
     - "In Review" - Keep as in_progress
     - "No change" - Leave status unchanged
   - multiSelect: false

5. **Handle user response**:

   - If "Done":
     ```bash
     bd update "$BEADS_ISSUE" --status="closed"
     bd jira sync --push
     rm -f .beads/current-issue
     rm -f .jira/current-ticket.cache
     ```
   - If "In Review":
     ```bash
     bd update "$BEADS_ISSUE" --status="in_progress"
     bd jira sync --push
     ```
   - If "No change": Just sync status
     ```bash
     bd jira sync --push
     ```

### Step 4: Terraform Cloud Integration (if applicable)

After the PR is merged, check if Terraform Cloud integration is needed:

1. **Check if /terraform directory exists**:

   ```bash
   if [[ -d "terraform" ]]; then
     # Terraform directory exists, check for TFC integration
   fi
   ```

2. **Check if TFC_TOKEN is set**:

   ```bash
   if [[ -z "${TFC_TOKEN:-}" ]]; then
     # No TFC token, skip TFC integration silently
     # (Don't bother user if they don't have TFC set up)
   fi
   ```

3. **If both conditions met, run TFC check script**:

   ```bash
   "${CLAUDE_PLUGIN_ROOT}/scripts/tfc-check.sh" 2>&1
   ```

   The script will:
   - Find the TFC organization from terraform config
   - Find workspaces linked to this GitHub repository
   - Identify the non-production workspace (by name pattern: `*-dev`, `*-staging`, `*-nonprod`, or without `-prod`)
   - Check if a run is in progress for the latest commit
   - Wait for planning to complete if still running
   - Report any errors from the plan

4. **Handle TFC script output**:

   If the script outputs `TFC_APPLY_AVAILABLE=true`, it means:
   - The plan succeeded
   - The workspace requires approval before apply
   - The user has permission to approve

   Use **AskUserQuestion**:
   - Header: "Terraform Apply"
   - Question: "Terraform plan succeeded for workspace '$TFC_WORKSPACE'. Would you like to approve the apply?"
   - Options:
     - "Yes, approve apply" - Approve and start the apply
     - "No, I'll review first" - Skip, user can review in TFC UI
   - multiSelect: false

5. **Handle user response**:

   - If "Yes, approve apply":
     ```bash
     "${CLAUDE_PLUGIN_ROOT}/scripts/tfc-approve.sh" "$TFC_RUN_ID" 2>&1
     ```
   - If "No, I'll review first":
     ```
     Output: "You can review and approve the run at: $TFC_RUN_URL"
     ```

### Step 5: Final Output

After the script completes successfully, remind the user:

```
Tip: Run /compact to consolidate conversation history.
```

## Script Exit Codes

| Code | Meaning                         | Action                                           |
| ---- | ------------------------------- | ------------------------------------------------ |
| 0    | Success                         | PR merged and cleaned up                         |
| 1    | General error                   | Show error message                               |
| 2    | No PR found                     | Tell user to create PR first                     |
| 3    | PR already merged               | Proceed with cleanup                             |
| 4    | PR closed without merge         | Inform user, no cleanup                          |
| 5    | Merge blocked                   | Script shows reason, user must fix               |
| 6    | Admin merge available           | Ask user for confirmation (see Step 2)           |

## Error Handling

The script handles all error cases and outputs clear messages. If the script fails:

- **Exit code 2**: "No PR found for this branch. Create a PR first before running /done"
- **Exit code 4**: "PR was closed without merging. No cleanup needed."
- **Exit code 5**: The script will have output what's blocking (failing checks, conflicts, etc.)
- **Exit code 6**: **Not an error** - Admin merge is available but requires user confirmation (see Step 2)

## Environment Variables

The script respects these environment variables:

- `CHECK_INTERVAL`: Seconds between status updates when waiting for checks (default: 60)
- `MAX_WAIT_TIME`: Maximum seconds to wait for checks (default: 3600 = 1 hour)
