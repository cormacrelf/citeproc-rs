#
# Build the example client
#

TARGET = examples/client

OBJS = examples/client.o

RPATH=$(PWD)/../../target/debug
CFLAGS = -I./include
LDFLAGS = -L$(RPATH) -Wl,-rpath,$(RPATH)
LIBS = -lciteproc_rs

# targets
DYLIB = $(RPATH)/libciteproc_rs.dylib
GEN_HEADER=include/citeproc_rs.h
RUST_FFI_SOURCES = $(wildcard src/*.rs) $(wildcard src/*/*.rs)

.PHONY: all
all: $(DYLIB) $(GEN_HEADER) $(TARGET)

$(GEN_HEADER): $(RUST_FFI_SOURCES)
	cbindgen -o $@
	@touch $@

$(DYLIB):
	cargo build -p citeproc-ffi

$(TARGET): $(OBJS) $(GEN_HEADER)
	$(CC) -o $(TARGET) $(OBJS) $(LDFLAGS) $(LIBS)

.PHONY: clean
clean:
	rm -f $(OBJS) $(TARGET)
	cargo clean -p citeproc-ffi
	rm -f $(GEN_HEADER)

.PHONY: git
git: clean $(GEN_HEADER)
