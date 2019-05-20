# cargo-play

`cargo-play` is a tool to help you running your Rust code file without manually setting up a Cargo project.

## Install

```
cargo install cargo-play
```

## Usage

Simply running `cargo play <files>` is sufficient. You can specify your external dependency at the
beginning of your file with the prefix `//#`. It accepts the same TOML syntax as in `Cargo.toml`.

## Example

```
$ cat serde_json.rs
//# serde_json = "*"

use serde_json::{Result, Value};

fn main() -> Result<()> {
    // Some JSON input data as a &str. Maybe this comes from the user.
    let data = r#"
        {
            "name": "John Doe",
            "age": 43,
            "phones": [
                "+44 1234567",
                "+44 2345678"
            ]
        }"#;

    // Parse the string of data into serde_json::Value.
    let v: Value = serde_json::from_str(data)?;

    // Access parts of the data by indexing with square brackets.
    println!("Please call {} at the number {}", v["name"], v["phones"][0]);

    Ok(())
}

$ cargo play serde_json.rs
    Updating crates.io index
   Compiling serde v1.0.91
   Compiling ryu v0.2.8
   Compiling itoa v0.4.4
   Compiling serde_json v1.0.39
   Compiling gvzcg8yviqmd_euq3xti4-zbkrs v0.1.0 (/var/folders/nq/608n9lcx02n_mzx33_3z5wyw0000gn/T/cargo-play.GVzCg8yviQmd_EUq3Xti4-ZbKRs)
    Finished dev [unoptimized + debuginfo] target(s) in 10.23s
     Running `/var/folders/nq/608n9lcx02n_mzx33_3z5wyw0000gn/T/cargo-play.GVzCg8yviQmd_EUq3Xti4-ZbKRs/target/debug/gvzcg8yviqmd_euq3xti4-zbkrs`
Please call "John Doe" at the number "+44 1234567"
```

## To Do

- [ ] Editor plugins
  - [ ] Vim
  - [ ] VS Code
- [ ] Toolchain supports

## Acknowledgements

This project is inspired by [play.rust-lang.org](https://play.rust-lang.org) and [RustPlayground](https://github.com/cmyr/RustPlayground).