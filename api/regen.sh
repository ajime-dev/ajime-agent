#!/bin/bash
# Regenerate OpenAPI clients
# This script would use openapi-generator to regenerate Rust clients

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "Regenerating OpenAPI clients..."

# In production, you would use:
# openapi-generator generate -i specs/agent-server/v01.yaml -g rust -o ../libs/openapi-server
# openapi-generator generate -i specs/backend-client/v01.yaml -g rust -o ../libs/openapi-client

echo "Note: Manual regeneration required. Update libs/openapi-* as needed."
echo "Done."
