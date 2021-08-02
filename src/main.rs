use std::alloc::{alloc, Layout};
use std::env;
use std::error::Error;
use std::fs;
use std::io::{self, Read, Write};

#[derive(Debug)]
enum Op {
    Add(usize),
    Sub(usize),
    Lft(usize),
    Rht(usize),
    Dot(usize),
    Com(usize),
    Lbr(usize),
    Rbr(usize),
}

use Op::*;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args();
    let stdout = io::stdout();
    let stdin = io::stdin();
    let mut reader = stdin.lock().bytes();
    let mut writer = stdout.lock();

    let filename = args.nth(1).expect("Usage: bfrs filename.b");

    let code = fs::read_to_string(&filename)?;
    let size = code.len();
    let mut bytes = code
        .bytes()
        .filter(|n| b"+-<>[].,".contains(n))
        .enumerate()
        .peekable();

    let mut program = Vec::with_capacity(size);
    let mut queue = Vec::with_capacity(size);

    let mut total_count: usize = 0;
    let mut count: usize = 0;
    let (_, mut count_char) = *bytes.peek().unwrap();
    for (i, chr) in bytes {
        if count_char != chr || count_char == b'[' || count_char == b']' {
            match count_char {
                b'+' => program.push(Add(count)),
                b'-' => program.push(Sub(count)),
                b'<' => program.push(Lft(count)),
                b'>' => program.push(Rht(count)),
                b'.' => program.push(Dot(count)),
                b',' => program.push(Com(count)),
                _ => {}
            };
            count_char = chr;
            // This fixes a bug caused by the program sometimes starting with `[`
            if count > 0 {
                total_count += count - 1;
            }
            count = 1;
        } else {
            count += 1;
        }

        if chr == b'[' {
            queue.push((program.len(), i - total_count));
        } else if chr == b']' {
            let (orig_index, new_index) = queue.pop().expect("Error: Unbalanced [ and ]");
            program.insert(orig_index, Lbr(i - total_count));
            program.push(Rbr(new_index))
        }
    }

    // Convert program to a slice, speeds up
    let program = program.as_slice();

    // Fixed memory of 2^16 cells
    // Making it dynamic increases the speed a bit
    // ```rs
    // let mut mem = vec![0u8; 1000];
    // //...
    // while mem.len() <= ptr {
    //     mem.push(0);
    // }
    // ```
    let mut mem = [0u8; 65536];
    let mut ptr = 0;

    let mut i = 0;
    while i < program.len() {
        // Safety: this range will always be within the string thanks to the above check
        match program[i] {
            Add(c) => mem[ptr] = mem[ptr].wrapping_add(c as u8),
            Sub(c) => mem[ptr] = mem[ptr].wrapping_add(255 - c as u8 + 1),
            Rht(c) => {
                ptr += c;
                if ptr >= 65536 {
                    return Err("Pointer out of bounds!".into());
                }
            }
            Lft(c) => {
                if ptr < c {
                    return Err("Pointer out of bounds!".into());
                }
                ptr -= c;
            }
            Dot(c) => {
                // This would be easier if I could just fucking do `[mem[ptr]; c]`
                if c > 1 {
                    let val;
                    // Safety: these all exist and conform to the requirements of each unsafe function
                    unsafe {
                        val = alloc(Layout::from_size_align_unchecked(c, 4));
                        val.write_bytes(mem[ptr], c);

                        writer.write(std::slice::from_raw_parts(val, c))?;
                    }
                } else {
                    writer.write(&[mem[ptr]])?;
                }
            }
            // All others will be overwritten, so we only need the last byte that will be read
            Com(c) => mem[ptr] = reader.nth(c - 1).unwrap_or(Ok(0))?,
            Lbr(pos) => {
                if mem[ptr] == 0 {
                    i = pos;
                }
            }
            Rbr(pos) => {
                if mem[ptr] != 0 {
                    i = pos;
                }
            }
        }

        i += 1;
    }

    Ok(())
}
