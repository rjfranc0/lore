pub fn run() {
    let manual = include_str!("help.txt");
    // use $PAGER if set, fall back to cat-like direct print
    let pager = std::env::var("PAGER").unwrap_or_else(|_| "less".to_string());
    if !pager.is_empty() {
        if let Ok(mut child) = std::process::Command::new(&pager)
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            if let Some(stdin) = child.stdin.take() {
                use std::io::Write;
                let mut w = stdin;
                let _ = w.write_all(manual.as_bytes());
            }
            let _ = child.wait();
            return;
        }
    }
    print!("{manual}");
}
