#!/usr/bin/env bash
# tfc-check.sh - Check Terraform Cloud workspace status after merge
#
# This script:
# 1. Finds the TFC organization from terraform config
# 2. Finds workspaces linked to this GitHub repository
# 3. Identifies the non-production workspace
# 4. Checks if a run is in progress for the latest commit
# 5. Waits for planning to complete
# 6. Reports status and offers approval if applicable
#
# Requires: TFC_TOKEN environment variable
#
# Exit codes:
#   0 - Success (plan complete, may have TFC_APPLY_AVAILABLE signal)
#   1 - Error or no TFC setup found
#   2 - Plan failed with errors

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${GREEN}✓${NC} $*"; }
log_warn() { echo -e "${YELLOW}⚠${NC} $*"; }
log_error() { echo -e "${RED}✗${NC} $*" >&2; }
log_status() { echo -e "${BLUE}⏳${NC} $*"; }

# Check prerequisites
if [[ -z "${TFC_TOKEN:-}" ]]; then
	exit 1 # Silent exit, no TFC token
fi

if [[ ! -d "terraform" ]]; then
	exit 1 # Silent exit, no terraform directory
fi

# TFC API base URL
TFC_API="https://app.terraform.io/api/v2"

# Make TFC API request
tfc_api() {
	local method="$1"
	local endpoint="$2"
	local data="${3:-}"

	local args=(-s -H "Authorization: Bearer ${TFC_TOKEN}" -H "Content-Type: application/vnd.api+json")

	if [[ -n "$data" ]]; then
		args+=(-d "$data")
	fi

	curl "${args[@]}" -X "$method" "${TFC_API}${endpoint}"
}

# Find TFC organization from terraform config
find_tfc_org() {
	local org=""

	# Try to find organization in terraform files
	for tf_file in terraform/*.tf terraform/**/*.tf; do
		[[ -f "$tf_file" ]] || continue

		# Look for cloud block with organization
		org=$(grep -A5 'cloud {' "$tf_file" 2>/dev/null | grep 'organization' | sed 's/.*=.*"\([^"]*\)".*/\1/' | head -1 || echo "")
		[[ -n "$org" ]] && break

		# Look for backend "remote" with organization
		org=$(grep -A10 'backend "remote"' "$tf_file" 2>/dev/null | grep 'organization' | sed 's/.*=.*"\([^"]*\)".*/\1/' | head -1 || echo "")
		[[ -n "$org" ]] && break
	done

	echo "$org"
}

# Get GitHub repo info
get_github_repo() {
	local remote_url
	remote_url=$(git remote get-url origin 2>/dev/null || echo "")

	# Extract owner/repo from URL
	if [[ "$remote_url" =~ github\.com[:/]([^/]+)/([^/.]+) ]]; then
		echo "${BASH_REMATCH[1]}/${BASH_REMATCH[2]}"
	fi
}

# Find workspaces linked to this repo
find_linked_workspaces() {
	local org="$1"
	local github_repo="$2"

	# List all workspaces in the organization
	local workspaces
	workspaces=$(tfc_api GET "/organizations/${org}/workspaces?page%5Bsize%5D=100" | jq -r '.data[] | @base64')

	local linked_workspaces=()

	for ws in $workspaces; do
		local ws_data
		ws_data=$(echo "$ws" | base64 -d)

		local ws_id ws_name vcs_repo
		ws_id=$(echo "$ws_data" | jq -r '.id')
		ws_name=$(echo "$ws_data" | jq -r '.attributes.name')
		vcs_repo=$(echo "$ws_data" | jq -r '.attributes["vcs-repo"].identifier // empty')

		# Check if workspace is linked to our repo
		if [[ "$vcs_repo" == "$github_repo" ]]; then
			echo "${ws_id}:${ws_name}"
		fi
	done
}

# Identify non-production workspace
find_nonprod_workspace() {
	local workspaces="$1"

	# Priority order for non-prod identification
	# 1. Contains -dev
	# 2. Contains -staging
	# 3. Contains -nonprod
	# 4. Does NOT contain -prod

	for pattern in "-dev" "-staging" "-nonprod"; do
		while IFS=: read -r ws_id ws_name; do
			if [[ "$ws_name" == *"$pattern"* ]]; then
				echo "${ws_id}:${ws_name}"
				return 0
			fi
		done <<<"$workspaces"
	done

	# Fall back to any workspace that doesn't have -prod
	while IFS=: read -r ws_id ws_name; do
		if [[ "$ws_name" != *"-prod"* && "$ws_name" != *"-production"* ]]; then
			echo "${ws_id}:${ws_name}"
			return 0
		fi
	done <<<"$workspaces"

	return 1
}

# Get the latest run for a workspace
get_latest_run() {
	local ws_id="$1"

	tfc_api GET "/workspaces/${ws_id}/runs?page%5Bsize%5D=1" | jq -r '.data[0] // empty'
}

# Wait for run to complete planning
wait_for_plan() {
	local run_id="$1"
	local max_wait=600 # 10 minutes
	local waited=0
	local interval=10

	while [[ $waited -lt $max_wait ]]; do
		local run_data
		run_data=$(tfc_api GET "/runs/${run_id}")

		local status
		status=$(echo "$run_data" | jq -r '.data.attributes.status')

		case "$status" in
		"pending" | "plan_queued" | "planning")
			log_status "Terraform plan in progress... ($((waited / 60))m ${((waited % 60))}s elapsed)"
			sleep $interval
			waited=$((waited + interval))
			;;
		"planned" | "planned_and_finished" | "cost_estimated" | "policy_checked")
			echo "$run_data"
			return 0
			;;
		"policy_soft_failed")
			# Soft policy failure, can still apply with override
			echo "$run_data"
			return 0
			;;
		"errored" | "canceled" | "force_canceled" | "discarded")
			echo "$run_data"
			return 2
			;;
		"applied" | "apply_queued" | "applying")
			log_info "Run is already applying or applied"
			echo "$run_data"
			return 0
			;;
		*)
			log_warn "Unknown run status: $status"
			echo "$run_data"
			return 0
			;;
		esac
	done

	log_error "Timed out waiting for plan to complete"
	return 1
}

# Check if user can approve the run
can_approve_run() {
	local run_id="$1"

	local actions
	actions=$(tfc_api GET "/runs/${run_id}/actions" 2>/dev/null || echo "{}")

	# Check if confirm action is available
	echo "$actions" | jq -e '.data.attributes["is-confirmable"] == true' >/dev/null 2>&1
}

# Get plan error message
get_plan_error() {
	local run_id="$1"

	local plan_id
	plan_id=$(tfc_api GET "/runs/${run_id}" | jq -r '.data.relationships.plan.data.id // empty')

	if [[ -n "$plan_id" ]]; then
		local plan_log
		plan_log=$(tfc_api GET "/plans/${plan_id}/json-output" 2>/dev/null | jq -r '.error_message // empty')

		if [[ -n "$plan_log" ]]; then
			echo "$plan_log"
			return
		fi

		# Try to get from plan logs
		local log_url
		log_url=$(tfc_api GET "/plans/${plan_id}" | jq -r '.data.attributes["log-read-url"] // empty')

		if [[ -n "$log_url" ]]; then
			# Get last 50 lines of log which usually contains the error
			curl -s "$log_url" | tail -50
		fi
	fi
}

# Main
main() {
	log_status "Checking Terraform Cloud status..."

	# Find TFC organization
	local org
	org=$(find_tfc_org)

	if [[ -z "$org" ]]; then
		log_warn "No Terraform Cloud organization found in terraform config"
		exit 1
	fi

	log_info "Found TFC organization: $org"

	# Get GitHub repo
	local github_repo
	github_repo=$(get_github_repo)

	if [[ -z "$github_repo" ]]; then
		log_error "Could not determine GitHub repository"
		exit 1
	fi

	log_info "GitHub repository: $github_repo"

	# Find linked workspaces
	local workspaces
	workspaces=$(find_linked_workspaces "$org" "$github_repo")

	if [[ -z "$workspaces" ]]; then
		log_warn "No TFC workspaces found linked to this repository"
		exit 1
	fi

	log_info "Found linked workspaces"

	# Find non-production workspace
	local nonprod_ws
	if ! nonprod_ws=$(find_nonprod_workspace "$workspaces"); then
		log_warn "Could not identify non-production workspace"
		exit 1
	fi

	local ws_id ws_name
	IFS=: read -r ws_id ws_name <<<"$nonprod_ws"

	log_info "Non-production workspace: $ws_name"

	# Get latest run
	local run_data
	run_data=$(get_latest_run "$ws_id")

	if [[ -z "$run_data" || "$run_data" == "null" ]]; then
		log_info "No runs found for workspace $ws_name"
		exit 0
	fi

	local run_id run_status
	run_id=$(echo "$run_data" | jq -r '.id')
	run_status=$(echo "$run_data" | jq -r '.attributes.status')

	log_info "Latest run: $run_id (status: $run_status)"

	# Check if run is related to recent commit
	local run_commit
	run_commit=$(echo "$run_data" | jq -r '.attributes["commit-sha"] // empty')
	local head_commit
	head_commit=$(git rev-parse HEAD 2>/dev/null | cut -c1-7)

	# Wait for plan if still in progress
	if [[ "$run_status" =~ ^(pending|plan_queued|planning)$ ]]; then
		log_status "Plan is in progress, waiting for completion..."

		if ! run_data=$(wait_for_plan "$run_id"); then
			local error_msg
			error_msg=$(get_plan_error "$run_id")

			log_error "Terraform plan failed!"
			echo ""
			echo "Error details:"
			echo "$error_msg"
			echo ""
			echo "TFC_RUN_URL=https://app.terraform.io/app/${org}/workspaces/${ws_name}/runs/${run_id}"

			exit 2
		fi

		run_status=$(echo "$run_data" | jq -r '.data.attributes.status')
	fi

	# Check final status
	case "$run_status" in
	"errored")
		local error_msg
		error_msg=$(get_plan_error "$run_id")

		log_error "Terraform plan errored!"
		echo ""
		echo "Error details:"
		echo "$error_msg"
		echo ""
		echo "TFC_RUN_URL=https://app.terraform.io/app/${org}/workspaces/${ws_name}/runs/${run_id}"

		exit 2
		;;
	"planned" | "cost_estimated" | "policy_checked" | "policy_soft_failed")
		log_info "Terraform plan completed successfully"

		# Check if approval is required and user can approve
		local needs_confirmation
		needs_confirmation=$(echo "$run_data" | jq -r '.attributes["is-confirmable"] // false')

		if [[ "$needs_confirmation" == "true" ]]; then
			if can_approve_run "$run_id"; then
				echo ""
				echo "TFC_APPLY_AVAILABLE=true"
				echo "TFC_RUN_ID=$run_id"
				echo "TFC_WORKSPACE=$ws_name"
				echo "TFC_ORG=$org"
				echo "TFC_RUN_URL=https://app.terraform.io/app/${org}/workspaces/${ws_name}/runs/${run_id}"
				echo ""
				log_info "Plan requires approval before apply. You can approve this run."
			else
				log_info "Plan requires approval but you don't have permission to approve."
				echo "TFC_RUN_URL=https://app.terraform.io/app/${org}/workspaces/${ws_name}/runs/${run_id}"
			fi
		else
			log_info "Auto-apply is enabled, run will apply automatically"
		fi
		;;
	"applied" | "planned_and_finished")
		log_info "Run has already completed"
		;;
	*)
		log_info "Run status: $run_status"
		;;
	esac

	exit 0
}

main "$@"
