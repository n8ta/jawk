# Mawk regex

Mawk implements a high performance regex library. This crates wraps it.

The underlying library is NOT thread safe when creating a regex so you may not call 
`Regex::new()` concurrently without risking a crash. It is up to the crate user to synchronize access to that function.