"""
Tachyon RAG Diagnostics - Verification tests for the vector search system.

This diagnostic script validates the RAG (Retrieval-Augmented Generation) pipeline
by executing representative semantic queries and verifying that results contain
expected subsystem keywords. Used to detect vector misalignment or configuration issues.

Usage:
    $ python test_query.py

Exit Codes:
    0: All tests passed (RAG fully operational)
    1: Initialization failed (database or model issues)
    2: Some tests failed (degraded RAG quality)

Test Strategy:
    - Uses heuristic keyword matching in file paths
    - Not strict equality; checks for subsystem relevance
    - Designed to catch major regressions, not subtle quality shifts
"""
import sys
import lancedb
from fastembed import TextEmbedding
import config

# =============================================================================
# TACHYON RAG DIAGNOSTICS
# =============================================================================

def run_diagnostics() -> None:
    """
    Execute diagnostic test suite for RAG vector search quality.

    Runs 3 representative queries spanning different subsystems (AVX-512, streaming,
    CLI) and validates that semantic search returns results from the expected modules.
    Uses simple heuristic: at least one result path must contain the expected keyword.

    Raises:
        Does not raise exceptions; errors are caught and reported with exit codes.
        Calls sys.exit() directly with appropriate code.

    Side Effects:
        - Connects to LanceDB and loads FastEmbed model
        - Prints test results to stdout with emoji indicators
        - Exits process with status code (0=success, 1=init fail, 2=degraded)

    Test Cases:
        1. AVX-512 query: "Where is the AVX-512 512-bit vector register compression implemented?"
           Expected: Results should contain 'avx512' in file path

        2. Streaming query: "How does the streaming hash state accumulate intermediate chunks?"
           Expected: Results should contain 'streaming' in file path

        3. CLI query: "Which file defines the clap arguments and parsing logic for the command line?"
           Expected: Results should contain 'cli' in file path

    Rationale:
        These queries represent the 3 major subsystems and test semantic understanding
        rather than keyword matching. If the embeddings are broken, queries will return
        irrelevant files and fail the keyword heuristic.

    Example Output:
        --- [ Running RAG System Diagnostics ] ---
        Initializing Database connection and FastEmbed ONNX Engine...
        Executing Semantic Code Searches...
          ✓ Test 1: 'Where is the AVX-512...' -> Found relevant module!
          ✓ Test 2: 'How does the streaming...' -> Found relevant module!
          ✓ Test 3: 'Which file defines...' -> Found relevant module!

        ------------------------------------------------
        ✅ RAG Engine Fully Operational (3/3 tests passed).
    """
    print("--- [ Running RAG System Diagnostics ] ---")
    print("Initializing Database connection and FastEmbed ONNX Engine...")

    try:
        db = lancedb.connect("lancedb_data")
        table = db.open_table(config.database.table_name)
        model = TextEmbedding(
            model_name=config.embedding.model_name,
            threads=config.embedding.threads
        )
    except Exception as e:
        print(f"❌ Diagnostic Failed (Initialization): {str(e)}")
        sys.exit(1)

    # -------------------------------------------------------------------------
    # Test Cases
    # -------------------------------------------------------------------------
    test_cases = [
        {
            "query": "Where is the AVX-512 512-bit vector register compression implemented?",
            "expected_keyword": "avx512"
        },
        {
            "query": "How does the streaming hash state accumulate intermediate chunks?",
            "expected_keyword": "streaming"
        },
        {
            "query": "Which file defines the clap arguments and parsing logic for the command line?",
            "expected_keyword": "cli"
        }
    ]

    passed_tests = 0

    print("Executing Semantic Code Searches...")
    for i, test in enumerate(test_cases, 1):
        try:
            query_vector = list(model.embed([test["query"]]))[0].tolist()
            results = table.search(query_vector).limit(3).to_list()
            
            # Simple heuristic: at least one result path should contain the expected subsystem keyword
            success = any(test["expected_keyword"] in r["file"].lower() for r in results)
            
            if success:
                print(f"  ✓ Test {i}: '{test['query']}' -> Found relevant module!")
                passed_tests += 1
            else:
                print(f"  ❌ Test {i}: '{test['query']}' -> Vector mismatch.")
        except Exception as e:
             print(f"  ❌ Test {i}: Query crashed -> {str(e)}")

    # -------------------------------------------------------------------------
    # Result
    # -------------------------------------------------------------------------
    print("\n------------------------------------------------")
    if passed_tests == len(test_cases):
        print(f"✅ RAG Engine Fully Operational ({passed_tests}/{len(test_cases)} tests passed).")
        sys.exit(0)
    else:
        print(f"⚠️ Warning: RAG output may be degraded ({passed_tests}/{len(test_cases)} tests passed).")
        sys.exit(2)

if __name__ == "__main__":
    run_diagnostics()
