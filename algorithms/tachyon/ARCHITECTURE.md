# Tachyon Cryptographic Architecture

Tachyon is designed as a **Hardware-Accelerated, Parallel Merkle-Tree Hash**. It uses hardware SIMD registers (AVX-512 / AES-NI) to process data in independent lanes before a final mathematical reduction step.

## 1. Dual-Path Routing
The algorithm routes inputs based on their length to minimize latency for small inputs while maximizing throughput for bulk data:

*   **Bulk Path AVX512 (>= 64 Bytes):** Utilizes a **4096-bit internal state** (8 × 512-bit AVX-512 registers or 32 × 128-bit AES-NI registers). Data is processed across 8 parallel lanes.
*   **Short Path AES-NI (< 64 Bytes):** Routes to a dedicated, low-latency AES-NI kernel. It operates on a **512-bit state** (4 × 128-bit registers) and utilizes precomputed constants for the default seed/key to bypass initialization overhead.

### Pipeline (Bulk Path per core simplified)


```mermaid
graph TD
    %% Input Node
    Input(["Input Data (>= 64 Bytes)"]) --> Distribute{Distribute to Registers}
    
    %% Parallel Processing Block
    subgraph SIMD [" "]
        direction TB
        
        %% Lane 0
        Distribute -- Reg 0 --> L0_In[State 0]
        L0_In --> P0(AES + Feed-Forward)
        P0 --> L0_Out[New State 0]
        
        %% Middle Lanes (Ellipsis)
        Distribute -- ... --> LX_In[...]
        LX_In --> PX(...)
        PX --> LX_Out[...]

        %% Lane 7
        Distribute -- Reg 7 --> L7_In[State 7]
        L7_In --> P7(AES + Feed-Forward)
        P7 --> L7_Out[New State 7]
    end
    
    %% Finalization
    L0_Out & LX_Out & L7_Out --> Final{Finalization}
    
    subgraph End [" "]
        direction TB
        Final --> Mix["Permute & Global Mix"]
        Mix --> Reduce["Tree Reduction (AES)"]
        Reduce --> GFMUL["Galois Field Multiply"]
        GFMUL --> Hash(["256-bit Hash"])
    end
    
    %% Link Styles
    linkStyle default stroke:#4a4a4a,stroke-width:2px;
    
    %% Styling
    style SIMD fill:none,stroke:none
    
    style Input fill:#263238,stroke:#37474f,color:#fff,shadow:false
    style Distribute fill:#263238,stroke:#37474f,color:#fff,shadow:false
    
    style L0_In fill:#1b5e20,stroke:#2e7d32,color:#fff,shadow:false
    style P0 fill:#ff6f00,stroke:#e65100,color:#fff,stroke-width:2px,shadow:false
    style L0_Out fill:#33691e,stroke:#558b2f,color:#fff,shadow:false
    
    style L7_In fill:#1b5e20,stroke:#2e7d32,color:#fff,shadow:false
    style P7 fill:#ff6f00,stroke:#e65100,color:#fff,stroke-width:2px,shadow:false
    style L7_Out fill:#33691e,stroke:#558b2f,color:#fff,shadow:false
    
    style End fill:none,stroke:none
    
    style Final fill:#01579b,stroke:#0277bd,color:#fff,shadow:false
    style Mix fill:#01579b,stroke:#0277bd,color:#fff,shadow:false
    style Reduce fill:#4a148c,stroke:#7b1fa2,color:#fff,shadow:false
    style GFMUL fill:#4a148c,stroke:#7b1fa2,color:#fff,shadow:false
    style Hash fill:#b71c1c,stroke:#c62828,color:#fff,shadow:false
```

### Pipeline (Short Path AES-NI simplified)

```mermaid
graph TD
    %% Input Node
    Input(["Input Data (< 64 Bytes)"]) --> Distribute{Distribute to Registers}
    
    %% Processing Block
    subgraph AESNI [" "]
        direction TB
        
        %% Lane 0
        Distribute -- Reg 0 --> L0_In[State 0]
        L0_In --> P0(AES + Feed-Forward)
        P0 --> L0_Out[New State 0]
        
        %% Middle Lanes (Ellipsis)
        Distribute -- Reg 1 & 2 --> LX_In[...]
        LX_In --> PX(...)
        PX --> LX_Out[...]

        %% Lane 3
        Distribute -- Reg 3 --> L3_In[State 3]
        L3_In --> P3(AES + Feed-Forward)
        P3 --> L3_Out[New State 3]
    end
    
    %% Finalization
    L0_Out & LX_Out & L3_Out --> Final{Finalization}
    
    subgraph End [" "]
        direction TB
        Final --> Reduce["Tree Reduction (AES)"]
        Reduce --> Hash(["256-bit Hash"])
    end
    
    %% Link Styles
    linkStyle default stroke:#4a4a4a,stroke-width:2px;
    
    %% Styling
    style AESNI fill:none,stroke:none
    
    style Input fill:#263238,stroke:#37474f,color:#fff,shadow:false
    style Distribute fill:#263238,stroke:#37474f,color:#fff,shadow:false
    
    style L0_In fill:#1b5e20,stroke:#2e7d32,color:#fff,shadow:false
    style P0 fill:#ff6f00,stroke:#e65100,color:#fff,stroke-width:2px,shadow:false
    style L0_Out fill:#33691e,stroke:#558b2f,color:#fff,shadow:false
    
    style L3_In fill:#1b5e20,stroke:#2e7d32,color:#fff,shadow:false
    style P3 fill:#ff6f00,stroke:#e65100,color:#fff,stroke-width:2px,shadow:false
    style L3_Out fill:#33691e,stroke:#558b2f,color:#fff,shadow:false
    
    style End fill:none,stroke:none
    
    style Final fill:#01579b,stroke:#0277bd,color:#fff,shadow:false
    style Reduce fill:#4a148c,stroke:#7b1fa2,color:#fff,shadow:false
    style Hash fill:#b71c1c,stroke:#c62828,color:#fff,shadow:false
```

> [!NOTE]
> **Zero-Cost CLMUL & Mixing:** For data < 64 bytes (using default `seed=0` and `key=None`), Tachyon skips runtime **Global Mix** and **Quadratic CLMUL**. Instead, it loads `SHORT_INIT` constants—an empty state where these expensive cryptographic steps were already pre-computed. This provides full mathematical hardening with **zero runtime cost**. *(Custom seeds or keys safely fall back to the full runtime pipeline).*

## 2. Parallel Tree-Hashing (Throughput simplified)
For inputs of 256 KiB or larger, Tachyon automatically switches to a parallel tree-hash structure. Independent chunks are hashed concurrently, and their intermediate states are aggregated into a final Merkle root.

```mermaid
graph BT
    classDef chunk fill:#f1faee,stroke:#457b9d,stroke-width:2px,color:#1d3557,rx:5,ry:5
    classDef hash fill:#e63946,stroke:#a8dadc,stroke-width:2px,color:#f1faee,rx:10,ry:10
    classDef final fill:#1d3557,stroke:#a8dadc,stroke-width:3px,color:#f1faee,rx:20,ry:20

    C1[256 KiB Chunk]:::chunk --> H1(Thread / Lane Hash):::hash
    C2[256 KiB Chunk]:::chunk --> H2(Thread / Lane Hash):::hash
    C3[256 KiB Chunk]:::chunk --> H3(Thread / Lane Hash):::hash
    C4[256 KiB Chunk]:::chunk --> H4(Thread / Lane Hash):::hash
    
    H1 --> Root{Merkle Root Aggregation}:::hash
    H2 --> Root
    H3 --> Root
    H4 --> Root
    
    Root --> Final([Master 256-bit Hash]):::final
```

## 3. Advanced Cryptographic Elements

To ensure deep mixing and resistance to standard algebraic attacks (despite being an experimental hash function), Tachyon integrates several specialized cryptographic constructs into its pipeline:

*   **Davies-Meyer Feed-Forward (Compression):** To prevent pre-image attacks and ensure the compression block is a mathematically non-invertible one-way function, Tachyon employs the Davies-Meyer construction. At the end of processing each block, the pre-compression state is XORed back into the finalized accumulator state (`acc ⊕ s`).
*   **Butterfly Network Diffusion & Stride-3 Infection (Compression):** During the core block compression phase, the 32 parallel tracks (distributed across 8 AVX-512 ZMM registers) undergo a multi-stage **Butterfly Network** cross-accumulator diffusion. A critical part of this is the **Stride-3 Cross-Infect** step: By using a stride of 3 across the 8 lanes (where `gcd(3, 8) = 1`), Tachyon achieves rigorous state feedback that guarantees full diameter-3 diffusion across all parallel lanes in just 3 rounds. The mixing uses asymmetric XOR and ADD operations to continuously disrupt linear characteristics.
*   **Quadratic CLMUL Hardening (Finalization):** To prevent algebraic shortcuts and length-extension attacks, the finalization phase introduces **Quadratic CLMUL Hardening** over GF(2)[x]. It uses independent constants for polynomial mixing via `VPCLMULQDQ`, followed by a self-multiplication step to create a degree ~254² quadratic polynomial. AES encryption barriers are then layered over the output to eliminate linear paths back to the original state.
*   **Asymmetric Tree Reduction:** When merging the 8 parallel accumulator states down to a single 256-bit output, Tachyon uses a non-linear tree merge (8 → 4 → 2 → 1). At each level, mathematically independent constants are injected to break structural symmetry between lanes.
*   **Multi-Round Key Absorption:** When initializing a keyed hash (`hash_keyed`) or MAC, the key is not merely XORed into the state. During initialization, the key is absorbed using 2 AES rounds with per-lane offset differentiation and Golden Ratio masking to break symmetric key duplication. During finalization, an additional 4 AES rounds inject the key using 4 entirely distinct permutation patterns (cross, inverted cross, direct, and halved) to ensure full key diffusion across all output bits.
*   **Nothing-Up-My-Sleeve Constants:** With the sole exception of the Golden Ratio ($\phi$), *all* of Tachyon's internal constants are derived directly from the fractional parts of the natural logarithm of prime numbers (e.g., `ln(2)`, `ln(3)`, `ln(11)`). This ensures they are generated without bias. You can re-derive and verify these values yourself using the provided Python scripts in the `scripts/` directory.


