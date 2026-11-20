//! cargo run --release -p script-js-spike --features quickjs,boa --example spike
//!     [-- --boa-memory JS | --probe qjs|boa BUDGET JS]
fn main() {
    let args: Vec<String> = std::env::args().collect();
    #[cfg(feature = "boa")]
    if args.get(1).map(String::as_str) == Some("--boa-memory") {
        for (k, v) in script_js_spike::boa::memory(&args[2]).0 {
            println!("{k}: {v}");
        }
        return;
    }
    if args.get(1).map(String::as_str) == Some("--probe") {
        println!(
            "{}",
            script_js_spike::probe(&args[2], &args[4], args[3].parse().unwrap())
        );
        return;
    }
    print!("{}", script_js_spike::run());
}
