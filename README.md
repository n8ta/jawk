## What is it?
An (INCOMPLETE) jit compiled awk (jawk) implementation leveraging GNU libjit. The goal is the to be the fastest awk for all programs.

In reality, it will never be faster for programs like `BEGIN { print "A" }` since an interpreter has less overhead. 
But for all programs >5ms (in say gawk or mawk) jawk aims to be faster.

## How to use

### Ubuntu:
```
sudo apt-get install autoconf pkg-config libtool flex bison automake make g++
cargo build --release
``` 

### Mac:
```
brew install autoconf automake libtool gcc
```

### Windows
For now you need to use WSL and follow the ubuntu instructions

### General:
```
cargo build
./target/debug/jawk '{ print "Some awk program!}" }' 
./target/debug/jawk -f run.awk some_file.txt
cargo run -- --debug 'BEGIN { print "this will print debug info including the AST and runtime calls" }'
```

## Todo:

1. Reading from stdin
2. Support for awk functions
    - Functions are mutually recursive but not first class. Global too. 
    - Cannot be declared within each other.
    - `function a() { b() }; function b() { a () };` is fine
3. Native math functions like sin, cos, etc, rand, srand (libjit supports many of these)
4. Native string functions gsub, index, length, match, split, sprintf, sub, substr, tolower, toupper
5. Regex expressions matched/not-matched (in JIT or runtime)
6. Array support
7. Redirect output to file
   - close() function
8. Missing Operators
   - expr in array a in b
9. Parsing / Lexing negative numbers
10. ARGV / ARGC and other ENV vars
11. Pattern Ranges 
13. Parser need to be able to print the where it was when shit went wrong and what happened
14. Do we actually need numeric strings???
15. The columns runtime needs to be much faster and lazier.
16. Make this compile on Windows!
17. Divide by 0 needs to print an error (tests for this will probably need to be bespoke)

## License
GNU Libjit is GPLv2. 
This project is MIT licensed.

## Running the tests

Install other awks to test against (they should be on your path with these exact names)
1. gawk (linux/mac you already have it)
2. [mawk](https://invisible-island.net/mawk/) - build from src
3. [goawk](https://github.com/benhoyt/goawk) - need the go toolchain, then go get
4. [onetrueawk](https://github.com/onetrueawk/awk) - super easy and fast build from src

Tests by default just check correctness against other awks and oracle result.

```
cargo test
```

### Perf tests
If you want to run perf tests set the env var "jperf" to "true" and do a `cargo build --release` and `cargo test -- --test-threads=1` first. This will test the speed of the release binary against other awks.
