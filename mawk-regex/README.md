# Mawk regex

Mawk implements a high performance regex library. This crates wraps it.

The underlying library is NOT thread safe when creating a regex so you may not call 
`Regex::new()` concurrently without risking a crash. It is up to the crate user to synchronize access to that function.

If the thread_safe feature is enabled the mawk regex code is wrapped in a global mutex. This will slow it down (I only use it for testing).