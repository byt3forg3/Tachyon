package main

import (
	"encoding/hex"
	"encoding/json"
	"fmt"
	"os"
	"tachyon" // Use the binding!
)

// TestVectors represents the JSON structure
type TestVectors struct {
	Vectors []struct {
		Name  string `json:"name"`
		Input string `json:"input"`
		Hash  string `json:"hash"`
	} `json:"vectors"`
}

func main() {
	// Load test vectors
	data, err := os.ReadFile("../../tests/test_vectors.json")
	if err != nil {
		fmt.Printf("❌ Failed to load test vectors: %v\n", err)
		os.Exit(1)
	}

	var vectors TestVectors
	if err := json.Unmarshal(data, &vectors); err != nil {
		fmt.Printf("❌ Failed to parse test vectors: %v\n", err)
		os.Exit(1)
	}

	fmt.Println("Testing Tachyon Go Binding...\n")

	for _, vec := range vectors.Vectors {
		fmt.Printf("\n[Test Case: %s]\n", vec.Name)

		// Expand placeholders to actual data
		var input []byte
		switch vec.Input {
		case "LARGE_1KB":
			input = make([]byte, 1024)
			for i := range input {
				input[i] = 0x41 // 'A'
			}
		case "MEDIUM_256_A":
			input = make([]byte, 256)
			for i := range input {
				input[i] = 0x41 // 'A'
			}
		case "HUGE_1MB":
			input = make([]byte, 1024*1024)
			for i := range input {
				input[i] = 0x41 // 'A'
			}
		case "EXACT_64_ZERO":
			input = make([]byte, 64) // zero-filled by default
		case "EXACT_512_ONE":
			input = make([]byte, 512)
			for i := range input {
				input[i] = 0x01
			}
		case "UNALIGNED_63_TWO":
			input = make([]byte, 63)
			for i := range input {
				input[i] = 0x02
			}
		default:
			input = []byte(vec.Input)
		}
		expected := vec.Hash

		// 1. Hash
		hash, err := tachyon.Hash(input)
		if err != nil {
			fmt.Printf("❌ Error: %v\n", err)
			os.Exit(1)
		}

		hexHash := hex.EncodeToString(hash)
		fmt.Printf("  Input len: %d\n", len(input))
		fmt.Printf("  Hash:      %s\n", hexHash)
		fmt.Printf("  Expected:  %s\n", expected)

		if hexHash != expected {
			fmt.Printf("❌ Hash mismatch for '%s'!\n", vec.Name)
			os.Exit(1)
		}
		fmt.Println("  ✓ Hash matches")

		// 2. Verify
		valid, err := tachyon.Verify(input, hash)
		if err != nil {
			fmt.Printf("❌ Verify Error: %v\n", err)
			os.Exit(1)
		}
		if !valid {
			fmt.Printf("❌ Verification failed for '%s'!\n", vec.Name)
			os.Exit(1)
		}
		fmt.Println("  ✓ Verification passed")

		// 3. Bad verify
		badHash := make([]byte, len(hash))
		copy(badHash, hash)
		badHash[0] ^= 0xFF

		invalid, _ := tachyon.Verify(input, badHash)
		if invalid {
			fmt.Printf("❌ Bad verification succeeded for '%s'!\n", vec.Name)
			os.Exit(1)
		}
		fmt.Println("  ✓ Bad hash rejected")
	}

	fmt.Println("\n✅ Go Binding OK (All vectors passed)")
}
