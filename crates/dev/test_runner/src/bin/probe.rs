//! `utw-runner-probe <mb> <code>`: the child the runner's tests wrap. It allocates and
//! touches `<mb>` MB, then exits with `<code>`.

use std::hint::black_box;

/// Parses the arguments, commits `<mb>` MB and exits with `<code>`.
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (mb, code) = parse(&args);
    let buffer = vec![1u8; mb << 20];
    // One read per page, so the allocation is committed rather than only reserved.
    let mut sum = 0u64;
    for page in (0..buffer.len()).step_by(4096) {
        sum += u64::from(buffer[page]);
    }
    black_box(sum);
    std::process::exit(code);
}

/// `<mb> <code>` from `args`; anything else gets the usage line and exit 2.
#[expect(clippy::print_stderr, reason = "the probe's usage goes to stderr")]
fn parse(args: &[String]) -> (usize, i32) {
    if let [mb, code] = args
        && let (Ok(mb), Ok(code)) = (mb.parse::<usize>(), code.parse::<i32>())
    {
        return (mb, code);
    }
    eprintln!("usage: utw-runner-probe <mb> <code>");
    std::process::exit(2);
}
