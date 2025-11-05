# faisync-rs

A small Rust crate to asynchronously (with tokio) read FASTA index (FAI) files (or from memory)  and obtain relevant sequences from specific regions from the associated FASTA

Also contains some quality of life functions, adding traits to the String type for reverse complement sequences

## Example usage -

```rust
use faisync::Fasta;

// Specifying None for the fai_path in from_path
// attempts to open a .fai file in same dir with same name
let mut fasta = Fasta::from_path("genome.fasta", None).await.expect("Unable to open");
let sequence = fasta.read_region("chr1", 100, 200).await.expect("Unable to read region");

println!("Sequence = {}", sequence);
println!("Reverse complement = {}", sequence.reverse_complement()");

// Whole tids can be read into memory to save IO access with read_tid()
// can be faster when continously hitting lots of small sequences from the same chromosome/tid
let contig = fasta.read_tid("chr1").await.expect("Unable to read region");

let sequence = contig.read_region(100, 200).unwrap();

println!("Sequence = {}", sequence);
println!("Reverse complement = {}", sequence.reverse_complement()");
```

## TODO -

 - Documentation!
