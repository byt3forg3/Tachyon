#!/usr/bin/env python3
"""
Tachyon MCP Server - Semantic code search via Model Context Protocol.
Powered by LanceDB and the official mcp SDK.

This server provides a single MCP tool `search_tachyon_code` that enables
semantic code search across the Tachyon Rust codebase using vector embeddings.

Example:
    Start the server:
    $ python mcp_server.py

    Or configure in Claude Code's MCP settings.
"""
import sys
import logging
from pathlib import Path
from typing import Any, Dict, Optional
import lancedb
from fastembed import TextEmbedding
from mcp.server.fastmcp import FastMCP
import config

# =============================================================================
# SETUP
# =============================================================================

logging.basicConfig(level=logging.ERROR)
logger = logging.getLogger("tachyon-mcp")

TACHYON_ROOT = Path(__file__).parent.parent
LANCEDB_DIR = Path(__file__).parent / "lancedb_data"

# =============================================================================
# SERVER INSTANCES
# =============================================================================

mcp = FastMCP("Tachyon Code Search")
db: Optional[lancedb.Connection] = None
model: Optional[TextEmbedding] = None

@mcp.tool()
def search_tachyon_code(query: str, n_results: int = None) -> str:
    """
    Semantic search in the Tachyon codebase.

    Finds Rust functions, modules, and code patterns by semantic meaning rather than
    keyword matching. Uses Nomic embeddings and LanceDB vector search with automatic
    result deduplication to provide diverse results across different files.

    Args:
        query: Natural language description of what you're looking for.
            Examples: "Where is CLMUL hardening implemented?", "AVX-512 compression"
        n_results: Maximum number of unique results to return (default: 5).
            Note: Internally fetches 3x this amount for deduplication.

    Returns:
        Formatted string containing search results with file paths, function names,
        similarity distances, and code previews. Returns error message on failure.

    Raises:
        Does not raise exceptions directly; all errors are caught and returned
        as error message strings for MCP protocol compatibility.

    Example:
        >>> search_tachyon_code("AVX-512 compression", n_results=3)
        Search: 'AVX-512 compression'
        Found 3 unique results:

        1. [algorithms/tachyon/src/kernels/avx512.rs] :: compress (Distance: 0.3421)
           File: algorithms/tachyon/src/kernels/avx512.rs
           Function: compress
           ...

    Implementation Notes:
        - Lazy initialization: Database and model are loaded on first call
        - Prepends "search_query: " to improve Nomic embedding retrieval quality
        - Deduplicates results to show only best match per file
        - Fetches n_results * 3 internally to allow for effective deduplication
    """
    global db, model

    # Use default from config if not specified
    if n_results is None:
        n_results = config.search.default_results

    # Initialize lazily on first search to keep boot fast
    if not db or not model:
        try:
            db = lancedb.connect(str(LANCEDB_DIR))
            # Fallback to checking the directory existence or using table_names
            if not (LANCEDB_DIR / f"{config.database.table_name}.lance").exists():
                return "Error: RAG not indexed. Please run setup.sh first."
            model = TextEmbedding(
                model_name=config.embedding.model_name,
                threads=config.embedding.threads
            )
        except Exception as e:
            return f"Error initializing Search: {str(e)}"
    
    try:
        # Prepend "search_query: " to improve Nomic embedding quality
        instruction_query = f"search_query: {query}"
        query_vector = list(model.embed([instruction_query]))[0].tolist()

        table = db.open_table(config.database.table_name)

        # Fetch N*multiplier results for deduplication (ensures diverse file coverage)
        raw_results = table.search(query_vector).limit(n_results * config.search.dedup_multiplier).to_list()

        if not raw_results:
            return "No matching code found in Tachyon."

        # Deduplicate: keep only best match per file
        seen_files = set()
        unique_results = []
        for r in raw_results:
            file_path = r.get("file")
            if file_path not in seen_files:
                seen_files.add(file_path)
                unique_results.append(r)
                if len(unique_results) >= n_results:
                    break
            
        output_lines = [f"Search: '{query}'", f"Found {len(unique_results)} unique results:\n"]
        
        for i, r in enumerate(unique_results, 1):
            file_path = r.get("file", "unknown")
            func_name = r.get("function", "N/A")
            distance = r.get("_distance", 0.0)
            text_preview = r.get("text", "")[:config.search.preview_chars] + "..."
            
            output_lines.append(f"{i}. [{file_path}] :: {func_name} (Distance: {distance:.4f})")
            output_lines.append(f"   {text_preview}\n")
            
        return "\n".join(output_lines)

    except Exception as e:
        # Return error as string for MCP protocol compatibility
        logger.error(f"Search failed: {e}")
        return f"Error during search: {str(e)}"

if __name__ == "__main__":
    logger.info("Starting Tachyon RAG MCP Server (LanceDB)")
    # Start the standard stdio server loop provided by fastmcp
    mcp.run()
