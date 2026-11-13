use std::io::Write as _;

fn main() -> std::io::Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if let Ok(path) = std::env::var("GNOMCODE_FAKE_USAGE_MARKER") {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        writeln!(file, "{}|{}", std::process::id(), args.join("|"))?;
    }
    if args == ["--version"] {
        println!(
            "{}",
            std::env::var("GNOMCODE_FAKE_USAGE_VERSION").unwrap_or_else(|_| "1.5.7".into())
        );
        return Ok(());
    }
    if args != ["--output-format", "text", "-p=/usage"] {
        std::process::exit(2);
    }
    match std::env::var("GNOMCODE_FAKE_USAGE_MODE")
        .unwrap_or_default()
        .as_str()
    {
        "hang" => std::thread::sleep(std::time::Duration::from_secs(30)),
        "large" => std::io::stdout().write_all(&vec![b'x'; 70_000])?,
        "fail" => std::process::exit(3),
        "malformed" => println!("not quota metadata"),
        _ => print!("{}", include_str!("usage.tsv")),
    }
    Ok(())
}
