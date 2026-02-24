package tachyon

import (
	"bytes"
	"testing"
)

func TestConstants(t *testing.T) {
	if DomainGeneric != 0 {
		t.Errorf("DomainGeneric = %d, want 0", DomainGeneric)
	}
	if DomainFileChecksum != 1 {
		t.Errorf("DomainFileChecksum = %d, want 1", DomainFileChecksum)
	}
	if DomainKeyDerivation != 2 {
		t.Errorf("DomainKeyDerivation = %d, want 2", DomainKeyDerivation)
	}
	if DomainMessageAuth != 3 {
		t.Errorf("DomainMessageAuth = %d, want 3", DomainMessageAuth)
	}
	if DomainDatabaseIndex != 4 {
		t.Errorf("DomainDatabaseIndex = %d, want 4", DomainDatabaseIndex)
	}
	if DomainContentAddressed != 5 {
		t.Errorf("DomainContentAddressed = %d, want 5", DomainContentAddressed)
	}
}

func TestHashWithDomain(t *testing.T) {
	data := []byte("test data")

	// Different domains produce different hashes
	h0, err := HashWithDomain(data, DomainGeneric)
	if err != nil {
		t.Fatalf("HashWithDomain failed: %v", err)
	}

	h1, err := HashWithDomain(data, DomainFileChecksum)
	if err != nil {
		t.Fatalf("HashWithDomain failed: %v", err)
	}

	h2, err := HashWithDomain(data, DomainKeyDerivation)
	if err != nil {
		t.Fatalf("HashWithDomain failed: %v", err)
	}

	if len(h0) != 32 || len(h1) != 32 || len(h2) != 32 {
		t.Error("Hash length should be 32 bytes")
	}

	if bytes.Equal(h0, h1) || bytes.Equal(h1, h2) || bytes.Equal(h0, h2) {
		t.Error("Different domains should produce different hashes")
	}

	// Same domain produces same hash
	h0Again, _ := HashWithDomain(data, DomainGeneric)
	if !bytes.Equal(h0, h0Again) {
		t.Error("Same domain should produce same hash")
	}
}

func TestHashKeyed(t *testing.T) {
	data := []byte("message")
	key := bytes.Repeat([]byte("k"), 32)

	mac, err := HashKeyed(data, key)
	if err != nil {
		t.Fatalf("HashKeyed failed: %v", err)
	}

	if len(mac) != 32 {
		t.Errorf("MAC length = %d, want 32", len(mac))
	}

	// Different keys produce different MACs
	key2 := bytes.Repeat([]byte("x"), 32)
	mac2, _ := HashKeyed(data, key2)
	if bytes.Equal(mac, mac2) {
		t.Error("Different keys should produce different MACs")
	}

	// Same key + data = same MAC
	macAgain, _ := HashKeyed(data, key)
	if !bytes.Equal(mac, macAgain) {
		t.Error("Same key and data should produce same MAC")
	}
}

func TestVerifyMAC(t *testing.T) {
	data := []byte("authenticate this")
	key := bytes.Repeat([]byte("s"), 32) // 32 bytes key

	mac, err := HashKeyed(data, key)
	if err != nil {
		t.Fatalf("HashKeyed failed: %v", err)
	}

	// Correct MAC verifies
	valid, err := VerifyMAC(data, key, mac)
	if err != nil {
		t.Fatalf("VerifyMAC failed: %v", err)
	}
	if !valid {
		t.Error("Valid MAC should verify")
	}

	// Wrong MAC fails
	wrongMAC := bytes.Repeat([]byte("x"), 32)
	valid, _ = VerifyMAC(data, key, wrongMAC)
	if valid {
		t.Error("Wrong MAC should not verify")
	}

	// Wrong key fails
	wrongKey := bytes.Repeat([]byte("w"), 32) // 32 bytes key
	valid, _ = VerifyMAC(data, wrongKey, mac)
	if valid {
		t.Error("Wrong key should not verify")
	}

	// Wrong data fails
	valid, _ = VerifyMAC([]byte("different data"), key, mac)
	if valid {
		t.Error("Wrong data should not verify")
	}
}

func TestDeriveKey(t *testing.T) {
	masterKey := bytes.Repeat([]byte("m"), 32)

	// Different contexts produce different keys
	k1, err := DeriveKey("app-v1", masterKey)
	if err != nil {
		t.Fatalf("DeriveKey failed: %v", err)
	}

	k2, _ := DeriveKey("app-v2", masterKey)
	k3, _ := DeriveKey("database", masterKey)

	if len(k1) != 32 || len(k2) != 32 || len(k3) != 32 {
		t.Error("Derived key length should be 32 bytes")
	}

	if bytes.Equal(k1, k2) || bytes.Equal(k2, k3) || bytes.Equal(k1, k3) {
		t.Error("Different contexts should produce different keys")
	}

	// Same context produces same key
	k1Again, _ := DeriveKey("app-v1", masterKey)
	if !bytes.Equal(k1, k1Again) {
		t.Error("Same context should produce same key")
	}
}

func TestHashSeeded(t *testing.T) {
	data := []byte("seeded data")
	seed1 := uint64(12345)
	seed2 := uint64(67890)

	h1, err := HashSeeded(data, seed1)
	if err != nil {
		t.Fatalf("HashSeeded failed: %v", err)
	}

	h2, err := HashSeeded(data, seed2)
	if err != nil {
		t.Fatalf("HashSeeded failed: %v", err)
	}

	h3, err := HashSeeded(data, seed1)
	if err != nil {
		t.Fatalf("HashSeeded failed: %v", err)
	}

	if len(h1) != 32 {
		t.Error("Hash length should be 32 bytes")
	}

	if bytes.Equal(h1, h2) {
		t.Error("Different seeds should produce different hashes")
	}

	if !bytes.Equal(h1, h3) {
		t.Error("Same seed should produce same hash")
	}

	// Test streaming seeded
	hasher := NewHasherSeeded(seed1)
	if hasher == nil {
		t.Fatal("NewHasherSeeded returned nil")
	}
	hasher.Update(data)
	sh1, err := hasher.Finalize()
	if err != nil {
		t.Fatalf("Streaming Finalize failed: %v", err)
	}

	if !bytes.Equal(h1, sh1) {
		t.Error("Streaming seeded hash should match oneshot seeded hash")
	}
}

func TestNewHasherWithDomain(t *testing.T) {
	data := []byte("streaming test data")

	// Hash with domain
	hasher1 := NewHasherWithDomain(DomainMessageAuth)
	if hasher1 == nil {
		t.Fatal("NewHasherWithDomain returned nil")
	}
	hasher1.Update(data[:10])
	hasher1.Update(data[10:])
	h1, err := hasher1.Finalize()
	if err != nil {
		t.Fatalf("Finalize failed: %v", err)
	}

	if len(h1) != 32 {
		t.Error("Hash length should be 32 bytes")
	}

	// Different domain produces different hash
	hasher2 := NewHasherWithDomain(DomainKeyDerivation)
	if hasher2 == nil {
		t.Fatal("NewHasherWithDomain returned nil")
	}
	hasher2.Update(data[:10])
	hasher2.Update(data[10:])
	h2, _ := hasher2.Finalize()

	if bytes.Equal(h1, h2) {
		t.Error("Different domains should produce different hashes")
	}

	// No domain (default)
	hasher3 := NewHasher()
	hasher3.Update(data)
	h3, _ := hasher3.Finalize()

	if bytes.Equal(h3, h1) || bytes.Equal(h3, h2) {
		t.Error("Default domain should differ from explicit domains")
	}
}

func TestErrorHandling(t *testing.T) {
	// Invalid domain
	_, err := HashWithDomain([]byte("test"), 99)
	if err == nil {
		t.Error("Invalid domain should return error")
	}

	// Wrong key size
	_, err = HashKeyed([]byte("data"), []byte("short"))
	if err == nil {
		t.Error("Wrong key size should return error")
	}

	// Wrong MAC size
	key := bytes.Repeat([]byte("k"), 32)
	_, err = VerifyMAC([]byte("data"), key, []byte("short"))
	if err == nil {
		t.Error("Wrong MAC size should return error")
	}
}
