# H1

> A nice webserver experience in Rust.


See [`examples/hello-world.rs`](examples/hello-world.rs) for a taste.


## Benchmark

```sh
# start the server
> cargo run --release --example techempower
# benchmark using wrk
> wrk -t12 -c400 -d15s --latency http://localhost:3000/plaintext
```
