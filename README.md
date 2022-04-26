# simple-rate-limiter

A simple rate limiter that minimizes contention caused by overactive clients.

## Design

This rate limiter is designed with the following goals:

1. Limited potential for a maliciously overactive client to degrade the ability to serve others

2. Minimal synchronization cost between CPU caches, while also being fairly accurate for smoothing out bursts of traffic

3. Memory overhead linear in the amount of active keys

## API

```rust
pub struct RateLimiter<K: Eq + Hash>;

impl<K: Eq + Hash> RateLimiter<K> {
    pub fn new(limit: usize, slots: usize) -> Arc<Self>;
    pub fn rotate_slots(&self);
    pub fn check_limited(&self, key: K) -> bool;
}
```

## Micro benchmarks (not to be trusted, measure your own applications)

Machine details: AMD® Ryzen 5 3600, x86_64-pc-linux-gnu, SMT enabled

<img src="benches/report/mostly-passing-violin.svg" style="background-color:white">
<img src="benches/report/mostly-failing-violin.svg" style="background-color:white">
<img src="benches/report/mostly-passing-contention-violin.svg" style="background-color:white">
<img src="benches/report/mostly-failing-contention-violin.svg" style="background-color:white">
