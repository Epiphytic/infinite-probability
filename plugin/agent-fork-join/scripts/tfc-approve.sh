#!/usr/bin/env bash
# tfc-approve.sh - Approve a Terraform Cloud run
#
# Usage: tfc-approve.sh <run-id>
#
# Requires: TFC_TOKEN environment variable

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

# Check arguments
if [[ $# -lt 1 ]]; then
	log_error "Usage: tfc-approve.sh <run-id>"
	exit 1
fi

RUN_ID="$1"

# Check for TFC token
if [[ -z "${TFC_TOKEN:-}" ]]; then
	log_error "TFC_TOKEN environment variable is not set"
	exit 1
fi

# TFC API base URL
TFC_API="https://app.terraform.io/api/v2"

# Make TFC API request
tfc_api() {
	local method="$1"
	local endpoint="$2"
	local data="${3:-}"

	local args=(-s -w "\n%{http_code}" -H "Authorization: Bearer ${TFC_TOKEN}" -H "Content-Type: application/vnd.api+json")

	if [[ -n "$data" ]]; then
		args+=(-d "$data")
	fi

	curl "${args[@]}" -X "$method" "${TFC_API}${endpoint}"
}

# Approve (confirm) the run
approve_run() {
	local run_id="$1"

	log_status "Approving Terraform run $run_id..."

	# The apply action requires a comment
	local payload='{"comment": "Approved via agent-fork-join /done command"}'

	local response
	response=$(tfc_api POST "/runs/${run_id}/actions/apply" "$payload")

	# Extract HTTP code (last line)
	local http_code
	http_code=$(echo "$response" | tail -1)
	local body
	body=$(echo "$response" | sed '$d')

	if [[ "$http_code" == "202" || "$http_code" == "200" ]]; then
		log_info "Run approved successfully!"
		log_info "Terraform apply is now in progress."

		# Get run details for URL
		local run_data
		run_data=$(tfc_api GET "/runs/${run_id}" | sed '$d')

		local ws_id org_name ws_name
		ws_id=$(echo "$run_data" | jq -r '.data.relationships.workspace.data.id')

		# Get workspace details
		local ws_data
		ws_data=$(tfc_api GET "/workspaces/${ws_id}" | sed '$d')
		ws_name=$(echo "$ws_data" | jq -r '.data.attributes.name')
		org_name=$(echo "$ws_data" | jq -r '.data.relationships.organization.data.id')

		echo ""
		echo "Monitor the apply at:"
		echo "https://app.terraform.io/app/${org_name}/workspaces/${ws_name}/runs/${run_id}"

		return 0
	else
		log_error "Failed to approve run (HTTP $http_code)"

		local error_msg
		error_msg=$(echo "$body" | jq -r '.errors[0].detail // .errors[0].title // "Unknown error"' 2>/dev/null || echo "$body")

		log_error "Error: $error_msg"
		return 1
	fi
}

# Main
main() {
	# First check if the run exists and is in a confirmable state
	local run_response
	run_response=$(tfc_api GET "/runs/${RUN_ID}")

	local http_code
	http_code=$(echo "$run_response" | tail -1)
	local run_data
	run_data=$(echo "$run_response" | sed '$d')

	if [[ "$http_code" != "200" ]]; then
		log_error "Could not fetch run $RUN_ID (HTTP $http_code)"
		exit 1
	fi

	local run_status
	run_status=$(echo "$run_data" | jq -r '.data.attributes.status')

	local is_confirmable
	is_confirmable=$(echo "$run_data" | jq -r '.data.attributes["is-confirmable"] // false')

	if [[ "$is_confirmable" != "true" ]]; then
		case "$run_status" in
		"applied" | "planned_and_finished")
			log_info "Run has already been applied or completed"
			;;
		"applying")
			log_info "Run is already applying"
			;;
		"errored" | "canceled" | "discarded")
			log_error "Run is in state '$run_status' and cannot be approved"
			exit 1
			;;
		*)
			log_error "Run is not in a confirmable state (status: $run_status)"
			exit 1
			;;
		esac
		exit 0
	fi

	# Approve the run
	if ! approve_run "$RUN_ID"; then
		exit 1
	fi

	exit 0
}

main "$@"
