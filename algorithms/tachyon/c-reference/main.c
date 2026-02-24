// Tachyon
// Copyright (c) byt3forg3
// Licensed under the MIT or Apache 2.0 License
// -------------------------------------------------------------------------

#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include "tachyon.h"
#include "tachyon_impl.h"

// =============================================================================
// CLI TOOL
// =============================================================================

int main(int argc, char **argv) {
    if (argc < 2) {
        printf("Usage: %s <string>\n", argv[0]);
        return 1;
    }

    uint8_t hash[HASH_SIZE];
    tachyon_hash((uint8_t*)argv[1], strlen(argv[1]), hash);

    printf("Tachyon Hash: ");
    for(int i=0; i<HASH_SIZE; i++) printf("%02x", hash[i]);
    printf("\n");

    return 0;
}
