// Package tachyon provides Go bindings for the Tachyon hash function.
//
// Tachyon is a high-performance cryptographically hardened hash function using AVX-512 + VAES.
//
// Example:
//
//	hash, err := tachyon.Hash([]byte("Hello, World!"))
//	if err != nil {
//	    log.Fatal(err)
//	}
//	fmt.Printf("%x\n", hash)
//
//	// Streaming for large data
//	hasher := tachyon.NewHasher()
//	hasher.Update([]byte("chunk 1"))
//	hasher.Update([]byte("chunk 2"))
//	result := hasher.Finalize()
package tachyon

/*
#cgo LDFLAGS: -L../../target/release -ltachyon
#include "../c/tachyon.h"
#include <stdlib.h>
*/
import "C"
import (
	"errors"
	"sync"
	"unsafe"
)

// ============================================================================
// DOMAIN CONSTANTS
// ============================================================================

const (
	DomainGeneric          = 0
	DomainFileChecksum     = 1
	DomainKeyDerivation    = 2
	DomainMessageAuth      = 3
	DomainDatabaseIndex    = 4
	DomainContentAddressed = 5
)

// ============================================================================
// ONE-SHOT API
// ============================================================================

// Hash computes the Tachyon hash of the input data.
//
// Returns a 32-byte hash or an error if the operation fails.
func Hash(data []byte) ([]byte, error) {
	hash := make([]byte, 32)
	outputPtr := (*C.uint8_t)(unsafe.Pointer(&hash[0]))

	var inputPtr *C.uint8_t
	if len(data) > 0 {
		inputPtr = (*C.uint8_t)(unsafe.Pointer(&data[0]))
	} else {
		var dummy byte
		inputPtr = (*C.uint8_t)(unsafe.Pointer(&dummy))
	}
	inputLen := C.size_t(len(data))

	res := C.tachyon_hash(inputPtr, inputLen, outputPtr)
	if res != 0 {
		return nil, errors.New("tachyon: internal error")
	}

	return hash, nil
}

// HashSeeded computes the Tachyon hash of the input data with a seed.
//
// Returns a 32-byte hash or an error if the operation fails.
func HashSeeded(data []byte, seed uint64) ([]byte, error) {
	hash := make([]byte, 32)
	outputPtr := (*C.uint8_t)(unsafe.Pointer(&hash[0]))

	var inputPtr *C.uint8_t
	if len(data) > 0 {
		inputPtr = (*C.uint8_t)(unsafe.Pointer(&data[0]))
	} else {
		var dummy byte
		inputPtr = (*C.uint8_t)(unsafe.Pointer(&dummy))
	}
	inputLen := C.size_t(len(data))

	res := C.tachyon_hash_seeded(inputPtr, inputLen, C.uint64_t(seed), outputPtr)
	if res != 0 {
		return nil, errors.New("tachyon: internal error")
	}

	return hash, nil
}

// Verify checks if data matches the expected hash in constant time.
//
// This function is timing-attack resistant and should be used for
// password verification, API key validation, etc.
func Verify(data []byte, expectedHash []byte) (bool, error) {
	if len(expectedHash) != 32 {
		return false, errors.New("tachyon: expected hash must be 32 bytes")
	}
	var inputPtr *C.uint8_t
	if len(data) > 0 {
		inputPtr = (*C.uint8_t)(unsafe.Pointer(&data[0]))
	} else {
		var dummy byte
		inputPtr = (*C.uint8_t)(unsafe.Pointer(&dummy))
	}
	inputLen := C.size_t(len(data))
	hashPtr := (*C.uint8_t)(unsafe.Pointer(&expectedHash[0]))

	res := C.tachyon_verify(inputPtr, inputLen, hashPtr)

	switch res {
	case 1:
		return true, nil
	case 0:
		return false, nil
	default:
		return false, errors.New("tachyon: internal error")
	}
}

// HashWithDomain computes hash with domain separation.
func HashWithDomain(data []byte, domain uint8) ([]byte, error) {
	if domain > 5 {
		return nil, errors.New("tachyon: domain must be 0-5")
	}
	hash := make([]byte, 32)
	outputPtr := (*C.uint8_t)(unsafe.Pointer(&hash[0]))

	var inputPtr *C.uint8_t
	if len(data) > 0 {
		inputPtr = (*C.uint8_t)(unsafe.Pointer(&data[0]))
	}
	inputLen := C.size_t(len(data))

	res := C.tachyon_hash_with_domain(inputPtr, inputLen, C.uint64_t(domain), outputPtr)
	if res != 0 {
		return nil, errors.New("tachyon: internal error")
	}

	return hash, nil
}

// HashKeyed computes keyed hash (MAC).
func HashKeyed(data []byte, key []byte) ([]byte, error) {
	if len(key) != 32 {
		return nil, errors.New("tachyon: key must be 32 bytes")
	}
	if len(data) == 0 {
		return nil, errors.New("tachyon: input cannot be empty")
	}

	mac := make([]byte, 32)
	inputPtr := (*C.uint8_t)(unsafe.Pointer(&data[0]))
	inputLen := C.size_t(len(data))
	keyPtr := (*C.uint8_t)(unsafe.Pointer(&key[0]))
	outputPtr := (*C.uint8_t)(unsafe.Pointer(&mac[0]))

	res := C.tachyon_hash_keyed(inputPtr, inputLen, keyPtr, outputPtr)
	if res != 0 {
		return nil, errors.New("tachyon: internal error")
	}

	return mac, nil
}

// VerifyMAC verifies keyed hash (MAC) in constant time.
func VerifyMAC(data []byte, key []byte, expectedMAC []byte) (bool, error) {
	if len(key) != 32 {
		return false, errors.New("tachyon: key must be 32 bytes")
	}
	if len(expectedMAC) != 32 {
		return false, errors.New("tachyon: expected MAC must be 32 bytes")
	}
	if len(data) == 0 {
		return false, errors.New("tachyon: input cannot be empty")
	}

	inputPtr := (*C.uint8_t)(unsafe.Pointer(&data[0]))
	inputLen := C.size_t(len(data))
	keyPtr := (*C.uint8_t)(unsafe.Pointer(&key[0]))
	macPtr := (*C.uint8_t)(unsafe.Pointer(&expectedMAC[0]))

	res := C.tachyon_verify_mac(inputPtr, inputLen, keyPtr, macPtr)

	switch res {
	case 1:
		return true, nil
	case 0:
		return false, nil
	default:
		return false, errors.New("tachyon: internal error")
	}
}

// DeriveKey derives cryptographic key from material.
func DeriveKey(context string, keyMaterial []byte) ([]byte, error) {
	if len(keyMaterial) != 32 {
		return nil, errors.New("tachyon: key material must be 32 bytes")
	}

	contextBytes := []byte(context)
	derived := make([]byte, 32)
	
	contextPtr := (*C.uint8_t)(unsafe.Pointer(&contextBytes[0]))
	contextLen := C.size_t(len(contextBytes))
	materialPtr := (*C.uint8_t)(unsafe.Pointer(&keyMaterial[0]))
	outputPtr := (*C.uint8_t)(unsafe.Pointer(&derived[0]))

	res := C.tachyon_derive_key(contextPtr, contextLen, materialPtr, outputPtr)
	if res != 0 {
		return nil, errors.New("tachyon: internal error or invalid UTF-8")
	}

	return derived, nil
}

// ============================================================================
// STREAMING API
// ============================================================================

// Hasher provides streaming hash computation for large data.
//
// Example:
//
//	hasher := tachyon.NewHasher()
//	hasher.Update([]byte("chunk 1"))
//	hasher.Update([]byte("chunk 2"))
//	hash := hasher.Finalize()
type Hasher struct {
	state     unsafe.Pointer
	finalized bool
	mu        sync.Mutex
}

// NewHasher creates a new streaming hasher.
//
// Returns nil if the hasher could not be created (e.g., CPU doesn't support AVX-512).
func NewHasher() *Hasher {
	state := C.tachyon_hasher_new()
	if state == nil {
		return nil
	}
	return &Hasher{state: state}
}

// NewHasherWithDomain creates a new streaming hasher with domain separation.
func NewHasherWithDomain(domain uint64) *Hasher {
	state := C.tachyon_hasher_new_with_domain(C.uint64_t(domain))
	if state == nil {
		return nil
	}
	return &Hasher{state: state}
}

// NewHasherSeeded creates a new streaming hasher with a seed.
func NewHasherSeeded(seed uint64) *Hasher {
	state := C.tachyon_hasher_new_seeded(C.uint64_t(seed))
	if state == nil {
		return nil
	}
	return &Hasher{state: state}
}

// Update adds data to the hasher.
//
// Can be called multiple times before Finalize.
// Returns an error if the hasher was already finalized.
func (h *Hasher) Update(data []byte) error {
	h.mu.Lock()
	defer h.mu.Unlock()

	if h.finalized {
		return errors.New("tachyon: hasher already finalized")
	}
	if len(data) == 0 {
		return nil // No-op for empty data
	}

	dataPtr := (*C.uint8_t)(unsafe.Pointer(&data[0]))
	dataLen := C.size_t(len(data))
	C.tachyon_hasher_update(h.state, dataPtr, dataLen)
	return nil
}

// Finalize returns the final hash and releases resources.
//
// The hasher cannot be used after calling Finalize.
func (h *Hasher) Finalize() ([]byte, error) {
	h.mu.Lock()
	defer h.mu.Unlock()

	if h.finalized {
		return nil, errors.New("tachyon: hasher already finalized")
	}

	hash := make([]byte, 32)
	outputPtr := (*C.uint8_t)(unsafe.Pointer(&hash[0]))
	C.tachyon_hasher_finalize(h.state, outputPtr)
	h.finalized = true
	h.state = nil
	return hash, nil
}

// Close releases resources without finalizing.
//
// Use this if you need to abort a hash computation.
func (h *Hasher) Close() {
	h.mu.Lock()
	defer h.mu.Unlock()

	if h.state != nil && !h.finalized {
		C.tachyon_hasher_free(h.state)
		h.state = nil
		h.finalized = true
	}
}
