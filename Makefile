rust:
	cargo build --release
all: rust
	gcc $(DEFINES) -o main main.c -L target/release -l rust_allocator -fsanitize=address

