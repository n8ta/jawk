## What is it?
A (WIP) bytecode multi-stack awk interpreter. The goal of rawk is the to be the fastest awk for all programs.

## What makes it unique?

### Typing (yes even in awk!)
rawk uses type inference to determine the types of variables: string, string numeric, number, array at compile time. 
This allows rawk to emit bytecode that is non-dynamic in many scenarios. For code like

```
{ a = 1; b = 2; print a + b; }
```
rawk emits this code for `print a + b` (Gscl means Global scalar)
```
   6 GsclNum(0)                              args: [[]]                 push: [[Num]]
   7 GsclNum(1)                              args: [[]]                 push: [[Num]]
   8 Add                                     args: [[Num, Num]]         push: [[Num]]
```
The add instruction knows its operands are numbers and will not need to check types at runtime.
```
pub fn add(vm: &mut VirtualMachine, ip: usize, _imm: Immed) -> usize {
    let rhs: f64 = vm.pop_num(); // pop from the numeric stack
    let lhs: f64 = vm.pop_num(); // again
    vm.push_num(lhs+rhs);        // add them and push
    ip + 1                       // advance
}
```
If the types of `a` and `b` are variable (like below)
```
   { if ($1) { a = 1; b = 2; } else { a = "1"; b = "2" } print (a + b); }
```
rawk has no significant advantage and will add two more bytecode ops to convert string -> number. For `print (a + b)` rawk emits
```
   16 GsclVar(0)                              args: [[]]                 push: [[Var]]
   17 VarToNum                                args: [[Var]]              push: [[Num]]
   18 GsclVar(1)                              args: [[]]                 push: [[Var]]
   19 VarToNum                                args: [[Var]]              push: [[Num]]
   20 Add                                     args: [[Num, Num]]         push: [[Num]]
```
Var means variable which is the stack of values whose type could be string/strnum/number and whose types need to be checked at runtime.

### Very fast IO
rawk uses a ring buffer to read from files without copying unless the data is needed. rawk's file reading is faster than all other
awks I am aware of. I have not yet optimized output so I have no idea how it compares. Here's a comparison of various awks
reading every line in a file storing it, and then printing the final value.

![io.png](assets%2Fio.png)

(onetrueawk is far to the right of this chart so I've omitted it)

## Todo:

1. Reading from stdin
2. Native string functions 
   1. index
   2. match
   3. split
   4. sprintf
3. Redirect output to file
   - close() function
4. Pattern Ranges 
5. The columns runtime should not duplicate work when the same field is looked up multiple times
6. The columns runtime should support assignment
7. Divide by 0 needs to print an error
8. All the builtin variables that are read only:
   1. ARGC (float)
   1. FILENAME (str)
   1. FNR (float)
   1. NF (float)
   1. NR (float)
   1. RLENGTH (float)
   1. RSTART (float)
9. Builtins that are read/write
    1. CONVFMT (str)
    1. FS (str)
    1. OFMT (str)
    1. OFS (str)
    1. ORS (str)
    1. RS (str)
    1. SUBSEP (str)
10. Builtins that are arrays (in this impl read only)
    1. ARGV
    1. ENVIRON

## License
Mawk is GPLv2 (./mawk-regex-sys/LICENSE)
Quick Drop Deque is MIT (./quick-drop-deque/LICENSE)
The combined project is GPLv2 

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
