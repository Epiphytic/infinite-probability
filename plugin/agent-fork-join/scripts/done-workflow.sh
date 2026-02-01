#!/usr/bin/env bash
# /done workflow script - Complete branch workflow with PR merge and cleanup
#
# This script handles:
# 1. Check PR status and mergeability
# 2. Wait for pending GitHub Actions with status updates
# 3. Merge the PR when ready (or signal for admin merge confirmation)
# 4. Output signals for beads/JIRA integration (handled by caller)
# 5. Switch to main and clean up local branch
#
# Usage:
#   done-workflow.sh              # Full workflow
#   done-workflow.sh --skip-merge # Skip merge, go straight to cleanup (after admin merge)
#
# Exit codes:
#   0 - Success (PR merged and cleaned up)
#   1 - Error (failed at some step)
#   2 - PR not found
#   3 - PR already merged (cleanup needed)
#   4 - PR closed without merge
#   5 - Merge blocked (conflicts, reviews required, etc.)
#   6 - Admin merge available (requires user confirmation)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source common utilities if available
if [[ -f "${SCRIPT_DIR}/../hooks/lib/common.sh" ]]; then
	source "${SCRIPT_DIR}/../hooks/lib/common.sh"
fi

# Configuration
CHECK_INTERVAL="${CHECK_INTERVAL:-60}" # seconds between status updates
MAX_WAIT_TIME="${MAX_WAIT_TIME:-3600}" # max wait time in seconds (1 hour default)

# Parse arguments
SKIP_MERGE=false
for arg in "$@"; do
	case "$arg" in
	--skip-merge)
		SKIP_MERGE=true
		;;
	esac
done

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
	echo -e "${GREEN}✓${NC} $*"
}

log_warn() {
	echo -e "${YELLOW}⚠${NC} $*"
}

log_error() {
	echo -e "${RED}✗${NC} $*"
}

log_status() {
	echo -e "${BLUE}⏳${NC} $*"
}

# Get current branch
get_current_branch() {
	git rev-parse --abbrev-ref HEAD 2>/dev/null
}

# Get default branch
get_default_branch() {
	git remote show origin 2>/dev/null | grep 'HEAD branch' | cut -d: -f2 | tr -d ' ' || echo "main"
}

# Check if branch is a feature branch
is_feature_branch() {
	local branch="$1"
	[[ "$branch" =~ ^(build|ci|docs|feat|fix|perf|refactor|test)/ ]]
}

# Get PR number for branch
get_pr_number() {
	local branch="$1"
	gh pr list --head "$branch" --state all --json number --jq '.[0].number' 2>/dev/null || echo ""
}

# Get PR state (OPEN, MERGED, CLOSED)
get_pr_state() {
	local pr_number="$1"
	gh pr view "$pr_number" --json state --jq '.state' 2>/dev/null || echo ""
}

# Check if PR was merged
was_pr_merged() {
	local pr_number="$1"
	local merged_at
	merged_at=$(gh pr view "$pr_number" --json mergedAt --jq '.mergedAt' 2>/dev/null || echo "null")
	[[ "$merged_at" != "null" && -n "$merged_at" ]]
}

# Get PR merge status and blockers
get_pr_merge_status() {
	local pr_number="$1"
	gh pr view "$pr_number" --json mergeable,mergeStateStatus,statusCheckRollup,reviewDecision 2>/dev/null
}

# Check if all required checks have passed
check_all_checks_passed() {
	local pr_number="$1"
	local status_json
	status_json=$(gh pr view "$pr_number" --json statusCheckRollup --jq '.statusCheckRollup' 2>/dev/null)

	if [[ -z "$status_json" || "$status_json" == "null" || "$status_json" == "[]" ]]; then
		# No checks configured
		return 0
	fi

	# Check if any checks are pending or failing
	local pending failing
	pending=$(echo "$status_json" | jq '[.[] | select(.status == "PENDING" or .status == "IN_PROGRESS" or .status == "QUEUED")] | length' 2>/dev/null || echo "0")
	failing=$(echo "$status_json" | jq '[.[] | select(.conclusion == "FAILURE" or .conclusion == "ERROR" or .conclusion == "CANCELLED")] | length' 2>/dev/null || echo "0")

	if [[ "$failing" -gt 0 ]]; then
		return 2 # Failing checks
	elif [[ "$pending" -gt 0 ]]; then
		return 1 # Pending checks
	else
		return 0 # All passed
	fi
}

# Get pending check names
get_pending_checks() {
	local pr_number="$1"
	gh pr view "$pr_number" --json statusCheckRollup --jq '.statusCheckRollup[] | select(.status == "PENDING" or .status == "IN_PROGRESS" or .status == "QUEUED") | .name' 2>/dev/null | tr '\n' ', ' | sed 's/,$//'
}

# Get failing check names
get_failing_checks() {
	local pr_number="$1"
	gh pr view "$pr_number" --json statusCheckRollup --jq '.statusCheckRollup[] | select(.conclusion == "FAILURE" or .conclusion == "ERROR") | .name' 2>/dev/null | tr '\n' ', ' | sed 's/,$//'
}

# Wait for all checks to complete
wait_for_checks() {
	local pr_number="$1"
	local waited=0

	while true; do
		check_all_checks_passed "$pr_number"
		local check_status=$?

		case $check_status in
		0)
			log_info "All checks passed!"
			return 0
			;;
		1)
			local pending_checks
			pending_checks=$(get_pending_checks "$pr_number")
			log_status "Waiting for checks to complete: ${pending_checks:-unknown}"

			if [[ $waited -ge $MAX_WAIT_TIME ]]; then
				log_error "Timed out waiting for checks after $((MAX_WAIT_TIME / 60)) minutes"
				return 1
			fi

			sleep "$CHECK_INTERVAL"
			waited=$((waited + CHECK_INTERVAL))
			log_status "Still waiting... ($((waited / 60)) minutes elapsed)"
			;;
		2)
			local failing_checks
			failing_checks=$(get_failing_checks "$pr_number")
			log_error "Some checks have failed: ${failing_checks:-unknown}"
			return 1
			;;
		esac
	done
}

# Check if PR is mergeable
is_pr_mergeable() {
	local pr_number="$1"
	local merge_status
	merge_status=$(gh pr view "$pr_number" --json mergeable --jq '.mergeable' 2>/dev/null || echo "")

	[[ "$merge_status" == "MERGEABLE" ]]
}

# Check review decision
get_review_decision() {
	local pr_number="$1"
	gh pr view "$pr_number" --json reviewDecision --jq '.reviewDecision' 2>/dev/null || echo ""
}

# Check if user has admin access (can bypass branch protection)
has_admin_access() {
	local pr_number="$1"
	# Try to check if we can use --admin flag
	gh pr view "$pr_number" --json viewerCanMergeAsAdmin --jq '.viewerCanMergeAsAdmin' 2>/dev/null || echo "false"
}

# Merge the PR
merge_pr() {
	local pr_number="$1"
	local use_admin="${2:-false}"

	if [[ "$use_admin" == "true" ]]; then
		log_info "Merging PR #${pr_number} with admin override..."
		gh pr merge "$pr_number" --squash --admin 2>&1
	else
		log_info "Merging PR #${pr_number}..."
		gh pr merge "$pr_number" --squash 2>&1
	fi
}

# Get beads issue info
get_beads_info() {
	local beads_dir=".beads"
	local current_issue_file="${beads_dir}/current-issue"

	if [[ ! -f "$current_issue_file" ]]; then
		return 1
	fi

	local issue_id
	issue_id=$(cat "$current_issue_file" 2>/dev/null | tr -d '[:space:]')

	if [[ -z "$issue_id" ]]; then
		return 1
	fi

	echo "$issue_id"
}

# Main workflow
main() {
	echo ""
	echo "=== Completing Branch Workflow ==="
	echo ""

	# Step 1: Get current state
	local current_branch default_branch
	current_branch=$(get_current_branch)
	default_branch=$(get_default_branch)

	log_info "Current branch: $current_branch"
	log_info "Default branch: $default_branch"

	# Check if on default branch already
	if [[ "$current_branch" == "$default_branch" ]]; then
		log_warn "Already on $default_branch branch. Nothing to do."
		exit 0
	fi

	# Check if feature branch
	if ! is_feature_branch "$current_branch"; then
		log_error "Not on a feature branch (expected feat/, fix/, etc.)"
		log_error "Current branch: $current_branch"
		exit 1
	fi

	# Step 2: Get PR info
	local pr_number
	pr_number=$(get_pr_number "$current_branch")

	if [[ -z "$pr_number" ]]; then
		log_error "No PR found for branch: $current_branch"
		log_error "Create a PR first before running /done"
		exit 2
	fi

	log_info "Found PR #${pr_number}"

	# Step 3: Check PR state (unless --skip-merge was passed)
	if [[ "$SKIP_MERGE" == "true" ]]; then
		log_info "Skipping merge check (--skip-merge flag)"
		# Verify PR is actually merged before proceeding
		if ! was_pr_merged "$pr_number"; then
			local pr_state
			pr_state=$(get_pr_state "$pr_number")
			if [[ "$pr_state" == "OPEN" ]]; then
				log_error "PR #${pr_number} is still open. Cannot skip merge."
				exit 1
			fi
		fi
		log_info "PR #${pr_number} is merged, proceeding to cleanup..."
	else
		local pr_state
		pr_state=$(get_pr_state "$pr_number")

		case "$pr_state" in
		"MERGED")
			log_info "PR #${pr_number} is already merged"
			# Skip to cleanup
			;;
		"CLOSED")
			if was_pr_merged "$pr_number"; then
				log_info "PR #${pr_number} was merged"
				# Skip to cleanup
			else
				log_error "PR #${pr_number} was closed without merging"
				exit 4
			fi
			;;
		"OPEN")
			log_info "PR #${pr_number} is open, checking mergeability..."

			# Step 4: Wait for pending checks
			log_status "Checking CI status..."
			if ! wait_for_checks "$pr_number"; then
				log_error "Cannot merge: checks failed or timed out"
				exit 5
			fi

			# Step 5: Check mergeability
			if ! is_pr_mergeable "$pr_number"; then
				log_error "PR is not mergeable (may have conflicts)"
				log_error "Please resolve conflicts on GitHub and try again"
				exit 5
			fi

			# Step 6: Check review decision
			local review_decision
			review_decision=$(get_review_decision "$pr_number")

			if [[ "$review_decision" == "REVIEW_REQUIRED" ]]; then
				log_warn "Review approval is required"

				# Check for admin access
				local can_admin
				can_admin=$(has_admin_access "$pr_number")

				if [[ "$can_admin" == "true" ]]; then
					# Output signals for caller to ask user for confirmation
					echo ""
					echo "ADMIN_MERGE_AVAILABLE=true"
					echo "ADMIN_MERGE_PR_NUMBER=$pr_number"
					echo "ADMIN_MERGE_BRANCH=$current_branch"
					echo "ADMIN_MERGE_DEFAULT_BRANCH=$default_branch"
					echo ""
					log_warn "You have admin access and can bypass approval requirements."
					log_warn "User confirmation is required before proceeding."
					echo ""
					echo "The Claude agent should use AskUserQuestion to ask:"
					echo "  Question: 'This PR requires approval but you have admin access. Would you like to merge anyway?'"
					echo "  Options:"
					echo "    - 'Yes, merge with admin override' - Merge bypassing approval requirements"
					echo "    - 'No, wait for approval' - Stop and wait for PR approval"
					echo ""
					exit 6
				else
					log_error "Review approval required and no admin access"
					log_error "Please get the PR reviewed and approved, then try again"
					exit 5
				fi
			elif [[ "$review_decision" == "CHANGES_REQUESTED" ]]; then
				log_error "Changes have been requested on this PR"
				log_error "Please address the review comments and try again"
				exit 5
			else
				# Merge the PR
				if ! merge_pr "$pr_number"; then
					log_error "Failed to merge PR"
					exit 5
				fi
			fi

			log_info "PR #${pr_number} merged successfully!"
			;;
		*)
			log_error "Unknown PR state: $pr_state"
			exit 1
			;;
		esac
	fi

	# Step 7: Output beads info for caller to handle
	local beads_issue
	if beads_issue=$(get_beads_info); then
		# Sync with JIRA to get latest status before outputting signals
		log_status "Syncing with JIRA..."
		bd jira sync 2>/dev/null || true

		echo ""
		echo "BEADS_ISSUE=$beads_issue"
		echo "BEADS_PR_NUMBER=$pr_number"
		echo "BEADS_DEFAULT_BRANCH=$default_branch"
		echo "BEADS_ISSUE_STATUS_QUESTION=true"
		echo ""
	fi

	# Step 8: Switch to default branch
	log_status "Switching to $default_branch branch..."

	# Stash any uncommitted changes
	if [[ -n "$(git status --porcelain 2>/dev/null)" ]]; then
		log_warn "Stashing uncommitted changes..."
		git stash push -m "Auto-stash before /done" 2>/dev/null || true
	fi

	git checkout "$default_branch" 2>&1
	log_info "Switched to $default_branch"

	# Step 9: Pull latest changes
	log_status "Pulling latest changes..."
	if ! git pull origin "$default_branch" 2>&1; then
		log_warn "Pull failed, trying to resolve..."
		git checkout --theirs . 2>/dev/null && git add -A 2>/dev/null || true
	fi
	log_info "Updated to latest"

	# Step 10: Delete local feature branch
	log_status "Deleting local branch: $current_branch..."
	git branch -D "$current_branch" 2>&1 || true
	git branch -dr "origin/$current_branch" 2>/dev/null || true
	log_info "Deleted local branch: $current_branch"

	# Step 11: Clean up session state
	rm -f .fork-join/current_session 2>/dev/null || true
	rm -f .fork-join/tracked_files.txt 2>/dev/null || true

	echo ""
	echo "=== Workflow Complete ==="
	echo ""
	echo "Tip: Run /compact to consolidate conversation history."
	echo ""

	exit 0
}

main "$@"
