## What is it?
An (INCOMPLETE) jit compiled awk (jawk) implementation leveraging GNU libjit. The goal is the to be the fastest awk for all programs.

## Performance

### Best case scenario for jawk
A long running purely mostly numeric program with little io. Not a great example for awk since awk program usually do a lot of IO. 
JIT provides the most benefit since JIT'ed math is vastly faster than interpreted. For this we use generating mandelbrot set as ascii art (see tt.x1 in the repo). This is highly numeric followed by a few quick prints.

![A histogram showing jawk is the vastly the fastest for this program](./assets/tt.x1.png)
![Mandelbrot set in ascii art style. Output of jawk](./assets/mandel.png)

That chart is not a mistake. A normal run of this program for jawk (on my 2015 laptop) is 170ms vs >2s for all other awks. 

### Worse case scenario for jawk (short program, JIT provides no benefit)
![A histogram showing onetrueawk as the fastest followed by jawk](./assets/begin.png)

Here we see jawk is doing okay but the interpreters are much closer and onetrueawk, the lightest of the interpreter, is generally slightly faster.

### Practical example

(TODO)

## Limitations

jawk doesn't support all of awk yet. Lots of builtins are missing and IO other than print is barely optimized. Files are read in there 
entirety up front (they should be streamed as needed). 

If gnu-lightjib the backing jit compiler doesn't have a backend for your system jawk will fallback to an interpreter
and lose most performance gains.

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
