#!/usr/bin/env bash
set -e

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "=================================================="
echo "   Tachyon RAG Setup (Python + LanceDB)           "
echo "=================================================="
echo ""

# Parse command-line arguments for non-interactive mode (AI setup)
case "$1" in
    --install)
        ACTION="FULL_INIT"
        ;;
    --update)
        ACTION="UPDATE_DB"
        ;;
    --reinstall)
        ACTION="FULL_INIT"
        echo "Removing existing installation..."
        rm -rf "$DIR/.venv"
        rm -rf "$DIR/lancedb_data"
        rm -f "$DIR/mcp_config.json"
        ;;
esac

# Check current status and show interactive menu ONLY if no args provided
if [ -z "$ACTION" ]; then
    VENV_EXISTS=false
    DB_EXISTS=false
    CONFIG_EXISTS=false

if [ -d "$DIR/.venv" ]; then
    VENV_EXISTS=true
fi

if [ -d "$DIR/lancedb_data" ]; then
    DB_EXISTS=true
fi

if [ -f "$DIR/mcp_config.json" ]; then
    CONFIG_EXISTS=true
fi

if [ "$VENV_EXISTS" = false ] || [ "$DB_EXISTS" = false ] || [ "$CONFIG_EXISTS" = false ]; then
    echo "Status: ❌ RAG System not fully initialized."
    echo ""
    echo "Options:"
    echo "  1) Full Initialization (Install Python deps + Build Vector DB)"
    echo "  q) Quit"
    echo ""
    read -p "Selection [1, q]: " choice
    
    case $choice in
        1)
            ACTION="FULL_INIT"
            ;;
        q|Q)
            echo "Cancelled."
            exit 0
            ;;
        *)
            echo "Invalid choice."
            exit 1
            ;;
    esac
else
    echo "Status: ✅ RAG System is active and installed."
    echo ""
    echo "Options:"
    echo "  1) Update Vector DB (Re-index Rust codebase)"
    echo "  2) Full Re-Install (Delete and rebuild Python venv + DB)"
    echo "  q) Quit (Do nothing)"
    echo ""
    read -p "Selection [1-2, q]: " choice
    
    case $choice in
        1)
            ACTION="UPDATE_DB"
            ;;
        2)
            ACTION="FULL_INIT"
            echo "Removing existing installation..."
            rm -rf "$DIR/.venv"
            rm -rf "$DIR/lancedb_data"
            rm -f "$DIR/mcp_config.json"
            ;;
        q|Q)
            echo "Cancelled."
            exit 0
            ;;
        *)
            echo "Invalid choice."
            exit 1
            ;;
    esac
fi
fi

# =============================================================================
# INSTALL DEPS (If full init)
# =============================================================================

if [ "$ACTION" = "FULL_INIT" ]; then
    echo ""
    echo "--- [ Setting up Virtual Environment ] ---"
    python3 -m venv "$DIR/.venv"
    source "$DIR/.venv/bin/activate"

    echo "--- [ Installing ML Dependencies ] ---"
    echo "Installing lightweight ONNX RAG dependencies (FastEmbed)..."
    pip install --upgrade pip
    pip install -r "$DIR/requirements.txt"
else
    # Just activate for the update step
    source "$DIR/.venv/bin/activate"
fi

# =============================================================================
# INDEXING
# =============================================================================

echo ""
echo "--- [ Indexing Codebase ] ---"
python "$DIR/indexer.py"

# =============================================================================
# MCP CONFIGURATION
# =============================================================================

if [ "$ACTION" = "FULL_INIT" ]; then
    echo ""
    echo "--- [ Generating MCP Config ] ---"
    cat <<EOF > "$DIR/mcp_config.json"
{
  "mcpServers": {
    "tachyon-rag": {
      "command": "$DIR/.venv/bin/python",
      "args": [
        "$DIR/mcp_server.py"
      ],
      "env": {}
    }
  }
}
EOF
    echo "Generated mcp_config.json"
fi

# =============================================================================
# VERIFICATION
# =============================================================================

echo ""
python "$DIR/test_query.py"

# =============================================================================
# COMPLETE
# =============================================================================

echo ""
echo "=================================================="
echo "   Setup Finished!                                "
echo "=================================================="
echo ""
echo "  NEXT STEPS:"
echo "  ------------------------------------------------"
echo "  ▶ Claude Code / VS Code / Cursor:"
echo "    Copy the contents of .tachyon-rag/mcp_config.json"
echo "    into your MCP client settings."
echo ""
echo "  ▶ Manual Re-indexing:"
echo "    Just run this setup.sh script again and select (1)!"
echo ""
