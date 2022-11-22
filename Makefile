rust-release:
	cargo build --release $(CARGO_FLAGS)
rust-debug:
	cargo build $(CARGO_FLAGS)
main: rust-release
	gcc $(DEFINES) -o main main.c -L target/release -l rust_allocator -fsanitize=address
crash: rust-debug
	gcc $(DEFINES) -o crash crash.c -L target/debug -l rust_allocator -fsanitize=address

multiple_allocs: rust-release
	gcc -O2 $(DEFINES) -o multiple_allocs ./multiple_allocations.c -L target/release -l rust_allocator -fsanitize=address

clean:
	rm crash main 
