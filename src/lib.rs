pub mod contig;
pub mod error;
pub mod fai;
pub mod fasta;
mod parser;

pub use fasta::*;
pub use fai::*;
pub use contig::*;

#[inline]
pub(crate) fn strip_gencode_style(id: &str) -> &str
{
	let bytes = id.as_bytes();
	for i in 0..bytes.len()
	{
		if bytes[i] == b'|'
		{
			return &id[..i];
		}
	}
	id
}
