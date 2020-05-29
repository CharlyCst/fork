# Fork

You are early to the party 🎉 Fork is still a work in progress, feel free to try it out though!

Fork is a language that compiles to WebAssembly, it aims at being **very portable** and **easy to integrate** with other wasm modules, possibly written in other languages.

To achieve these goals, the following features are currently explored:

- **Very small runtime**, exploring a Rust-style automatic memory management system (no GC)
- **Two module levels**: Fork-level imports/exports and WASM-level imports/exports
- Introduce the notion of host **Runtime**, allowing to expose interfaces with multiple underneath implementations using host runtime specific hooks (starting with Web and WASI)

## Trying Fork

Fork is still a work in progress and currently lacks some major features, but still you can try it out.

First clone this repository and build it

```bash
git clone git@github.com:CharlyCst/fork.git
cd fork
cargo build
```

Then write a Fork program:

```rust
export fun Main(a i32, b i32) i32 {
    if b == 0 {
        return 1
    }

    let n = b
    let x = a
    let acc = 1

    while n > 1 {
        if n % 2 == 1 {
            acc = acc * x
        }
        x = x * x
        n = n / 2
    }
    return x * acc
}
```

Compile it

```bash
cargo run -- pow.frk pow.wasm
```

And run it with your favorite WASM runtime, for instance [Wastime](https://github.com/bytecodealliance/wasmtime)

```bash
wasmtime pow.wasm 5 3
125
```

