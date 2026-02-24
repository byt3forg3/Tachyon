CC     = gcc
AR     = ar
ARFLAGS = rcs

# Common flags: optimise, warn, no -march=native.
# The C source files use GCC function-attribute targeting
# (TARGET_AVX512 / TARGET_AESNI) to enable SIMD per-function at
# compile time. The runtime dispatcher in tachyon_dispatcher.c then selects
# the best available backend via CPUID at startup.  Using -march=native
# here would (a) break the build on non-AVX-512 machines and (b) allow
# the compiler to silently vectorise the portable fallback, making it
# harder to verify its correctness independently.
CFLAGS_COMMON = -O3 -Wall -Wextra -Wno-unused-function

# Per-backend SIMD flags (only what each file actually uses)
CFLAGS_AVX512  = $(CFLAGS_COMMON) -mavx512f -mavx512bw -mvaes -mvpclmulqdq -maes -msse4.1 -mpclmul
CFLAGS_AESNI   = $(CFLAGS_COMMON) -maes -msse4.1 -mpclmul
CFLAGS_PORT    = $(CFLAGS_COMMON)
CFLAGS_GENERIC = $(CFLAGS_COMMON)

LIB_NAME = libtachyon.a

MAIN_SRC = main.c
MAIN_EXE = tachyon_ref

.PHONY: all clean lib

all: lib

lib: $(LIB_NAME)

$(LIB_NAME): tachyon_dispatcher.o tachyon_avx512.o tachyon_aesni.o tachyon_portable.o
	$(AR) $(ARFLAGS) $@ $^

tachyon_avx512.o: tachyon_avx512.c tachyon.h tachyon_impl.h
	$(CC) $(CFLAGS_AVX512) -c -o $@ $<

tachyon_aesni.o: tachyon_aesni.c tachyon.h tachyon_impl.h
	$(CC) $(CFLAGS_AESNI) -c -o $@ $<

tachyon_portable.o: tachyon_portable.c tachyon.h tachyon_impl.h
	$(CC) $(CFLAGS_PORT) -c -o $@ $<

tachyon_dispatcher.o: tachyon_dispatcher.c tachyon.h tachyon_impl.h
	$(CC) $(CFLAGS_GENERIC) -c -o $@ $<

$(MAIN_EXE): $(MAIN_SRC) $(LIB_NAME)
	$(CC) $(CFLAGS_GENERIC) -o $@ $^

clean:
	rm -f *.o $(LIB_NAME) $(MAIN_EXE)
