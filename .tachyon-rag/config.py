"""
Tachyon RAG Configuration - Central config for indexer and MCP server.

Modify these values to customize RAG behavior without changing code.
"""
from dataclasses import dataclass
from typing import List

# =============================================================================
# EMBEDDING MODEL
# =============================================================================

@dataclass
class EmbeddingConfig:
    """Configuration for FastEmbed ONNX model."""
    model_name: str = "nomic-ai/nomic-embed-text-v1.5"  # 384-dim embeddings, 8192 token context
    threads: int = 4  # CPU threads for ONNX runtime
    batch_size: int = 16  # Batch size for embedding generation (lower = less RAM)


# =============================================================================
# INDEXING
# =============================================================================

@dataclass
class IndexingConfig:
    """Configuration for code indexing."""
    rust_patterns: List[str] = None  # Glob patterns for Rust files
    ignore_patterns: List[str] = None  # Patterns to exclude
    code_preview_max_chars: int = 2000  # Max chars per code chunk

    def __post_init__(self):
        if self.rust_patterns is None:
            self.rust_patterns = [
                "algorithms/tachyon/src/**/*.rs",
                "algorithms/tachyon-zero/src/**/*.rs",
                "cli/src/**/*.rs",
            ]
        if self.ignore_patterns is None:
            self.ignore_patterns = [
                "**/target/**",
                "**/tests/**",  # Optional: skip test files
            ]


# =============================================================================
# SEARCH
# =============================================================================

@dataclass
class SearchConfig:
    """Configuration for semantic search."""
    default_results: int = 5  # Default number of results
    dedup_multiplier: int = 3  # Fetch N*multiplier for deduplication
    preview_chars: int = 300  # Characters to show in result preview


# =============================================================================
# DATABASE
# =============================================================================

@dataclass
class DatabaseConfig:
    """Configuration for LanceDB."""
    table_name: str = "tachyon_code"


# =============================================================================
# GLOBAL CONFIG
# =============================================================================

# Instantiate configs with defaults
embedding = EmbeddingConfig()
indexing = IndexingConfig()
search = SearchConfig()
database = DatabaseConfig()
