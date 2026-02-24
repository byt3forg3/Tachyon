# Official Test Vectors (8-Way ILP Architecture)

These vectors verify the correctness of the Tachyon implementation across languages and architectures.

## Canonical Hashes

| Scenario              | Input (Description)       | Hash (Hex)                                                         |
| :---                  | :---                      | :---                                                               |
| **Empty**             | `""` (0 bytes)            | `7f3485746a9ec855ec3ff1c8287e6c6cfbfa454a8bfa3dd71c3c3e5b39e7c549` |
| **Basic**             | `"abc"` (3 bytes)         | `3138c10ba15fe7d7fad8c7fc380474a0be7737a4e6296d246304ed767903e85b` |
| **Small**             | `"Tachyon"` (7 bytes)     | `120b887e8501bf2a342d397cc46d43b1796502ad75232e7f4c555379cef8c120` |
| **Medium**            | 256 x `'A'` (0x41)        | `bafe91fc7d73b8dadc19d0605fe3279762f67ea7f0f4e0ffb9c89634b112ce4d` |
| **Large**             | 1024 x `'A'` (0x41)       | `f14c3aeee98faa9f5c38f08c76f479d425f39da9b277743eff6c576f0470d509` |
| **Huge**              | 1024*1024 x `'A'` (1MB)   | `7693207f8983d9b991278d951cd4986589a5ffe611c05ee3011426b34dcc4689` |
| **Exact Block 64**    | 64 x `0x00`               | `860f861c54b613d87c45430644af0f59af86da8fd6c1ea77d27d3856951b795c` |
| **Exact Block 512**   | 512 x `0x01`              | `7011e32a0dbda6bda8be77b21a87399bfaa3a0d0114c25a9c14087b0750c4853` |
| **Unaligned 63**      | 63 x `0x02`               | `9e97ee668990325ac2189a2ce25e1f37d95177546bbf65cbe7b0ad8610978964` |

## Notes
- To verify the Rust implementation and print the hashes, run: `cargo test --test vectors -- --nocapture`

