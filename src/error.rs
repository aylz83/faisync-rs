use thiserror::Error;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Error, Debug)]
pub enum Error
{
	#[error("No FAIDX index")]
	NoFAIDX,
	#[error("Unable to parse FAIDX line")]
	ParseError,
	#[error("Unable to read FASTA region due to region specified being invalid")]
	InvalidRegion,
	#[error("IO error: {0}")]
	Io(#[from] std::io::Error),
}
