use std::{
    fs::OpenOptions, io::Result,
    os::unix::fs::OpenOptionsExt,
};

const CHUNK_SIZE: u64 = 4096 * 1;

// `O_DIRECT` requires all reads and writes
// to be aligned to the block device's block
// size. 4096 might not be the best, or even
// a valid one, for yours!
#[repr(align(4096))]
struct Aligned([u8; CHUNK_SIZE as usize]);

fn main() -> Result<()> {
    // start the ring
    let mut config = rio::Config::default();
    config.print_profile_on_drop = true;
    let ring = config.start().expect("create uring");

    // open output file, with `O_DIRECT` set
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .custom_flags(libc::O_DIRECT)
        .open("file")
        .expect("open file");

    let out_buf = Aligned([42; CHUNK_SIZE as usize]);
    let out_slice: &[u8] = &out_buf.0;

    let mut in_buf = Aligned([42; CHUNK_SIZE as usize]);
    let in_slice: &mut [u8] = &mut in_buf.0;

    let mut completions = vec![];

    const SIZE: u64 = 10 * 1024 * 256;

    println!("writing");
    let pre = std::time::Instant::now();

    for i in 0..SIZE {
        let at = i * CHUNK_SIZE;

        // By setting the `Link` order,
        // we specify that the following
        // read should happen after this
        // write.
        let write = ring.write_at_ordered(
            &file,
            &out_slice,
            at,
            rio::Ordering::None,
        );
        completions.push(write);
    }

    let post_submit = std::time::Instant::now();

    for completion in completions.into_iter() {
        completion.wait()?;
    }

    let post_complete = std::time::Instant::now();

    dbg!(post_submit - pre, post_complete - post_submit);

    println!("reading sequential");
    let pre = std::time::Instant::now();
    let mut completions = vec![];

    for i in 0..SIZE {
        let at = i * CHUNK_SIZE;

        let read = ring.read_at(&file, &in_slice, at);
        completions.push(read);
    }

    let post_submit = std::time::Instant::now();

    for completion in completions.into_iter() {
        completion.wait()?;
    }

    let post_complete = std::time::Instant::now();

    dbg!(post_submit - pre, post_complete - post_submit);

    println!("reading random 1000 - o_direct");
    use rand::{Rng, SeedableRng};

    let mut rng = rand::rngs::SmallRng::from_entropy();

    let pre = std::time::Instant::now();
    let mut completions = vec![];

    for i in 0..256 * 1000 {
        let at = rng.gen_range(0, SIZE) * CHUNK_SIZE;

        let read = ring.read_at(&file, &in_slice, at);
        completions.push(read);
    }

    let post_submit = std::time::Instant::now();

    for completion in completions.into_iter() {
        completion.wait()?;
    }

    let post_complete = std::time::Instant::now();

    dbg!(post_submit - pre, post_complete - post_submit);

    drop(file);

    println!("reading random 1000 - regular");

    let mut rng = rand::rngs::SmallRng::from_entropy();
    let file = std::fs::File::open("file")?;
    let buffer = &mut [0u8; 32];

    let pre = std::time::Instant::now();
    let mut completions = vec![];

    for i in 0..256 * 1000 {
        let at = rng.gen_range(0, SIZE) * CHUNK_SIZE;

        let read = ring.read_at(&file, &in_slice, at);
        completions.push(read);
    }

    let post_submit = std::time::Instant::now();

    for completion in completions.into_iter() {
        completion.wait()?;
    }

    let post_complete = std::time::Instant::now();

    dbg!(post_submit - pre, post_complete - post_submit);

    Ok(())
}
