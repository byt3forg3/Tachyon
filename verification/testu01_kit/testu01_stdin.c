#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <unistd.h>
#include <string.h>
#include "TestU01.h"

// Global buffer for reading
#define BUFFER_SIZE 4096
static uint8_t buffer[BUFFER_SIZE];
static size_t buffer_pos = BUFFER_SIZE; // Force initial read

// Function to refill buffer from stdin
static void refill_buffer() {
    size_t bytes_read = fread(buffer, 1, BUFFER_SIZE, stdin);
    if (bytes_read < BUFFER_SIZE) {
        if (feof(stdin)) {
            fprintf(stderr, "Error: End of stream reached (stdin exhausted).\n");
            exit(1);
        }
        if (ferror(stdin)) {
            perror("Error reading from stdin");
            exit(1);
        }
    }
    buffer_pos = 0;
}

// Generator function for TestU01 (returns 32-bit unsigned integer)
unsigned int stdin_generator(void) {
    if (buffer_pos + 4 > BUFFER_SIZE) {
        refill_buffer();
    }

    uint32_t val;
    memcpy(&val, &buffer[buffer_pos], 4);
    buffer_pos += 4;
    return val;
}

int main(int argc, char *argv[]) {
    // Unbuffered output for seeing progress
    setvbuf(stdout, NULL, _IONBF, 0);

    // Initial check if data is flowing
    if (isatty(fileno(stdin))) {
        fprintf(stderr, "Usage: ./generator | %s\n", argv[0]);
        fprintf(stderr, "Error: Standard input is a terminal. Please pipe data into this program.\n");
        return 1;
    }

    // Create external generator using our function
    unif01_Gen *gen = unif01_CreateExternGenBits("Stdin Stream", stdin_generator);

    printf("Starting TestU01 BigCrush on stdin stream...\n");
    bbattery_BigCrush(gen);

    unif01_DeleteExternGenBits(gen);
    return 0;
}
