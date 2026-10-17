// serde_json (preserve_order) against lsp/json.rs's contract: repeated keys, the nesting teq takes (512), a
// lone surrogate (teq reads U+FFFD).
fn main() {
    let dup: serde_json::Value = serde_json::from_str(r#"{"z":1,"a":2,"z":3}"#).unwrap();
    println!("repeated-keys {} z={}", dup, dup["z"]);
    for depth in [128usize, 129, 512] {
        let text = format!("{}{}", "[".repeat(depth), "]".repeat(depth));
        let r: Result<serde_json::Value, _> = serde_json::from_str(&text);
        println!("depth {depth}: {}", match r { Ok(_) => "read".to_string(), Err(e) => format!("refused ({e})") });
    }
    let r: Result<serde_json::Value, _> = serde_json::from_str(r#""\ud800x""#);
    println!("lone-surrogate: {}", match r { Ok(v) => format!("read {v}"), Err(e) => format!("refused ({e})") });
}
