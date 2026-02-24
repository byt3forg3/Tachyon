#!/usr/bin/env python3
"""
Tachyon Code Indexer - Converts Rust code into vector embeddings for RAG.
Powered by LanceDB and Tree-sitter.
"""
import os
import re
from pathlib import Path
from typing import List, Dict, Any
import lancedb
import structlog
from fastembed import TextEmbedding
import tree_sitter_rust as tsrust
from tree_sitter import Language, Parser, Node
import config

logger = structlog.get_logger()

# =============================================================================
# PATHS
# =============================================================================

TACHYON_ROOT = Path(__file__).parent.parent
LANCEDB_DIR = Path(__file__).parent / "lancedb_data"

def extract_items(rust_code_bytes: bytes, file_path: str, parser: Parser) -> List[Dict[str, Any]]:
    """
    Extract functions, constants, statics, and module documentation from Rust code.

    Uses tree-sitter AST parsing to walk the Rust syntax tree and extract:
    1. Module-level documentation comments (//! and /*! ... */)
    2. Function definitions with their doc comments (///)
    3. Constant and static definitions with inline and doc comments

    Args:
        rust_code_bytes: Raw UTF-8 encoded Rust source code as bytes.
            Must be bytes (not str) for tree-sitter compatibility.
        file_path: Relative path to the file being parsed (e.g., "algorithms/tachyon/src/lib.rs").
            Used for generating unique item IDs and tracking provenance.
        parser: Pre-configured tree-sitter Parser instance with Rust language loaded.
            Must have Language(tsrust.language()) set before calling.

    Returns:
        List of dictionaries representing extracted code items. Each dict contains:
            - 'id': Unique identifier (format: "file_path::item_name")
            - 'text': Searchable content including docs + code preview
            - 'file': Original file path
            - 'function': Item name (or "[module]" for module-level docs)
            - 'type': Item type ('module', 'function', 'const', 'static')

    Example:
        >>> parser = Parser(Language(tsrust.language()))
        >>> code = b'/// Hash function\\npub fn hash(data: &[u8]) -> [u8; 32] { ... }'
        >>> items = extract_items(code, "src/lib.rs", parser)
        >>> len(items)
        1
        >>> items[0]['function']
        'hash'

    Implementation Details:
        - Module docs are collected first by scanning top-level comments
        - Stops at first non-comment/non-doc item to avoid false matches
        - Doc comments are extracted by walking backwards through siblings
        - Code previews are truncated at 2000 chars to prevent token overflow
        - Uses recursive nested function `find_items()` for AST traversal

    See Also:
        - find_items(): Recursive helper for traversing the AST
        - Tree-sitter Rust grammar: https://github.com/tree-sitter/tree-sitter-rust
    """
    chunks = []
    
    # Parse the code into an AST
    tree = parser.parse(rust_code_bytes)
    root_node = tree.root_node
    
    # 1. Collect module-level docs first
    # Stop at first code item to avoid capturing inner comments
    module_docs = []
    for child in root_node.children:
        if child.type in ('line_comment', 'block_comment'):
            text = rust_code_bytes[child.start_byte:child.end_byte].decode('utf-8')
            if text.startswith('//!'):
                module_docs.append(text.removeprefix('//!').strip())
            elif text.startswith('/*!') and text.endswith('*/'):
                doc_content = text[3:-2].strip()
                cleaned_lines = [line.strip().removeprefix('*').strip() for line in doc_content.splitlines()]
                module_docs.append("\n".join(cleaned_lines).strip())
        elif child.type in ('function_item', 'struct_item', 'const_item', 'static_item', 'impl_item', 'trait_item'):
            break
            
    if module_docs:
        doc_text = "\n".join(module_docs)
        chunks.append({
            "id": f"{file_path}::[module]",
            "text": f"File: {file_path}\nModule Documentation:\n{doc_text}".strip(),
            "file": file_path,
            "function": "[module]",
            "type": "module"
        })
    
    # Simple recursive function to find all function, const, static definitions
    def find_items(node: Node, result: List[Dict[str, Any]]) -> None:
        """
        Recursively traverse AST to find and extract code items.

        This nested helper function performs depth-first traversal of the tree-sitter
        AST, identifying function_item, const_item, and static_item nodes. For each
        item found, it extracts the name, documentation comments, and source code.

        Args:
            node: Current tree-sitter Node being examined.
            result: Mutable list accumulating extracted items (modified in-place).

        Returns:
            None. Modifies `result` list in-place by appending item dictionaries.

        Algorithm:
            1. Iterate through all children of current node
            2. If child is a target item type (function/const/static):
               a. Extract item name from AST
               b. Walk backwards through siblings to collect doc comments
               c. For const/static: check for inline trailing comments
               d. Extract raw source code (truncate if >2000 chars)
               e. Format searchable text combining docs + code
               f. Append to result list
               g. Recurse into child (handles nested items in const blocks)
            3. For other node types: recurse normally

        Doc Comment Handling:
            - Leading doc comments (///) are collected by walking prev_sibling
            - Block doc comments (/** ... */) are cleaned of asterisks
            - Stops at non-comment siblings (ignores attribute_item like #[test])
            - Inline comments for const/static checked via next_sibling
        """
        for child in node.children:
            if child.type in ('function_item', 'const_item', 'static_item'):
                # Find the item name
                name_node = child.child_by_field_name('name')
                item_name = "unknown"
                if name_node:
                    item_name = rust_code_bytes[name_node.start_byte:name_node.end_byte].decode('utf-8')
                
                # Extract docs by walking backwards through siblings
                docs = []
                curr = child.prev_sibling
                while curr is not None:
                    if curr.type in ('line_comment', 'block_comment'):
                        text = rust_code_bytes[curr.start_byte:curr.end_byte].decode('utf-8')
                        if text.startswith('///'):
                            docs.insert(0, text.removeprefix('///').strip())
                        elif text.startswith('/**') and text.endswith('*/'):
                            doc_content = text[3:-2].strip()
                            cleaned_lines = [line.strip().removeprefix('*').strip() for line in doc_content.splitlines()]
                            docs.insert(0, "\n".join(cleaned_lines).strip())
                    elif curr.type == 'attribute_item':
                        pass  # Skip #[test], #[inline], etc.
                    else:
                        break
                    curr = curr.prev_sibling
                
                # Check for inline trailing comments (e.g., "const X: u32 = 5; // comment")
                if child.type in ('const_item', 'static_item'):
                    nex = child.next_sibling
                    if nex is not None and nex.type == 'line_comment':
                        if nex.start_point[0] == child.end_point[0]:  # Same line
                            text = rust_code_bytes[nex.start_byte:nex.end_byte].decode('utf-8')
                            docs.append(text.removeprefix('//').strip())
                
                # Extract the raw code
                raw_code = rust_code_bytes[child.start_byte:child.end_byte].decode('utf-8')
                max_chars = config.indexing.code_preview_max_chars
                code_preview = raw_code[:max_chars] + ("\n... (truncated)" if len(raw_code) > max_chars else "")
                
                doc_text = "\n".join(docs)
                item_type_str = child.type.split('_')[0] # 'function', 'const', 'static'
                
                searchable = f"""
File: {file_path}
{item_type_str.capitalize()}: {item_name}

Documentation:
{doc_text if doc_text else 'No documentation provided.'}

Code:
{code_preview}
""".strip()

                result.append({
                    "id": f"{file_path}::{item_name}",
                    "text": searchable,
                    "file": file_path,
                    "function": item_name,
                    "type": item_type_str
                })
                
                # Still recurse in case of nested functions inside const blocks etc.
                find_items(child, result)
                
            else:
                # Normal recursion for other nodes
                find_items(child, result)

    find_items(root_node, chunks)
    return chunks

def index_codebase() -> None:
    """
    Index the entire Tachyon Rust codebase into LanceDB vector database.

    Performs complete RAG indexing pipeline:
    1. Initialize tree-sitter Rust parser
    2. Discover .rs files matching RUST_PATTERNS
    3. Parse each file and extract items (functions, consts, module docs)
    4. Generate vector embeddings using FastEmbed ONNX model
    5. Store in LanceDB with metadata for retrieval

    Raises:
        Exception: If tree-sitter parser initialization fails
        IOError: If source files cannot be read
        RuntimeError: If LanceDB connection or table creation fails

    Side Effects:
        - Creates/overwrites lancedb_data/ directory
        - Downloads FastEmbed model on first run (~500MB ONNX file)
        - Drops and recreates 'tachyon_code' table if it exists
        - Prints progress information to stdout
        - Uses 4 CPU threads for embedding generation
        - Processes embeddings in batches of 16 to prevent OOM

    Performance:
        - ~1-2 minutes for full Tachyon codebase on typical hardware
        - Memory usage peaks during embedding generation (~2-4 GB)
        - Uses batch_size=16 to balance speed vs memory consumption

    Example Output:
        Tachyon RAG Indexer (Tree-sitter + LanceDB)
        ============================================================
        Initializing Rust AST Parser...
        Discovering .rs files...
          Found 47 valid source files.
          - algorithms/tachyon/src/lib.rs: 12 items
          ...
        Total items extracted: 342
        Loading lightweight FastEmbed ONNX model: nomic-ai/nomic-embed-text-v1.5...
        Generating vectors for code...
          Processing in small batches to save memory...
          Embedded 342/342 items...
        Constructing LanceDB vector table in lancedb_data...

        Indexing complete! Database ready at lancedb_data
    """
    print("Tachyon RAG Indexer (Tree-sitter + LanceDB)")
    print("============================================================")

    # =========================================================================
    # 1. SETUP AST PARSER
    # =========================================================================
    print("Initializing Rust AST Parser...")
    RUST_LANGUAGE = Language(tsrust.language())
    parser = Parser(RUST_LANGUAGE)
    
    # =========================================================================
    # 2. COLLECT FILES
    # =========================================================================
    print("Discovering .rs files...")
    rust_files = []
    for pattern in config.indexing.rust_patterns:
        rust_files.extend(TACHYON_ROOT.glob(pattern))

    rust_files = [f for f in rust_files if not any(f.match(ignore) for ignore in config.indexing.ignore_patterns)]
    print(f"  Found {len(rust_files)} valid source files.")

    # =========================================================================
    # 3. PARSE AND EXTRACT
    # =========================================================================
    all_chunks = []
    for rust_file in rust_files:
        rel_path = str(rust_file.relative_to(TACHYON_ROOT))
        try:
            code_bytes = rust_file.read_bytes()
            chunks = extract_items(code_bytes, rel_path, parser)
            all_chunks.extend(chunks)
            print(f"  - {rel_path}: {len(chunks)} items")
        except Exception as e:
            # Continue on parse failure (e.g., WIP code, tree-sitter version mismatch)
            logger.error("Failed to parse", file=rel_path, error=str(e))

    print(f"\nTotal items extracted: {len(all_chunks)}")

    # =========================================================================
    # 4. GENERATE EMBEDDINGS (FastEmbed / ONNX)
    # =========================================================================
    print(f"Loading lightweight FastEmbed ONNX model: {config.embedding.model_name}...")
    model = TextEmbedding(
        model_name=config.embedding.model_name,
        threads=config.embedding.threads
    )

    print("Generating vectors for code...")
    texts = [c["text"] for c in all_chunks]
    total = len(texts)

    # Set batch_size to prevent OOM (default 256 is too memory-hungry)
    print("  Processing in small batches to save memory...")
    embeddings_generator = model.embed(texts, batch_size=config.embedding.batch_size)

    embeddings = []
    for i, emb in enumerate(embeddings_generator, 1):
        embeddings.append(emb)
        print(f"  Embedded {i}/{total} items...", end="\r" if i < total else "\n", flush=True)

    # =========================================================================
    # 5. LANCEDB STORAGE
    # =========================================================================
    print(f"Constructing LanceDB vector table in {LANCEDB_DIR}...")
    import pyarrow as pa
    
    # Prepare Data for LanceDB/Arrow formats
    data = []
    for i, c in enumerate(all_chunks):
        row = {
            "id": c["id"],
            "vector": embeddings[i].tolist(),
            "text": c["text"],
            "file": c["file"],
            "function": c["function"],
        }
        data.append(row)

    # Connect and overwrite
    db = lancedb.connect(str(LANCEDB_DIR))
    if config.database.table_name in db.table_names():
        db.drop_table(config.database.table_name)

    db.create_table(config.database.table_name, data=data)

    print(f"\nIndexing complete! Database ready at {LANCEDB_DIR}")

if __name__ == "__main__":
    import logging
    logging.basicConfig(level=logging.ERROR)
    structlog.configure()
    index_codebase()
