use tokio::{fs::File, io::{AsyncRead, AsyncBufReadExt, BufReader}};
use std::collections::HashMap;
use std::path::Path;

use crate::error;
use crate::parser::parse_fai_line;

#[derive(Debug, Clone)]
pub struct FaiEntry
{
	pub name: String,
	pub length: u64,
	pub offset: u64,
	pub line_bases: u64,
	pub line_width: u64,
}

pub struct FaiIndex
{
	pub entries: HashMap<String, FaiEntry>,
}

impl FaiIndex
{
	pub async fn from_path<P>(path: P) -> error::Result<Self>
	where
		P: AsRef<Path>,
	{
		let file = File::open(path).await?;
		let reader = BufReader::new(file);
		Self::from_reader(reader).await
	}

	pub async fn from_reader<R>(reader: R) -> error::Result<Self>
	where
		R: AsyncRead + std::marker::Send + std::marker::Unpin,
	{
		let async_reader = BufReader::new(reader);
		let mut lines = async_reader.lines();

		let mut entries = HashMap::new();

		while let Some(line) = lines.next_line().await?
		{
			let (_, entry) =
				parse_fai_line(&(line + "\n")).map_err(|_| error::Error::ParseError)?;
			entries.insert(entry.name.clone(), entry);
		}

		Ok(Self { entries })
	}

	pub fn get_region_offsets(&self, chr: &str, start: u64, end: u64) -> Option<(u64, u64)>
	{
		let entry = self.entries.get(chr)?;

		if start >= entry.length || end > entry.length || start >= end
		{
			return None;
		}

		// Number of full lines before `start`
		let start_line = start / entry.line_bases;
		let start_offset_in_line = start % entry.line_bases;
		let start_file_offset = entry.offset + start_line * entry.line_width + start_offset_in_line;

		// Number of full lines before `end`
		let end_line = end / entry.line_bases;
		let end_offset_in_line = end % entry.line_bases;
		let end_file_offset = entry.offset + end_line * entry.line_width + end_offset_in_line;

		Some((start_file_offset, end_file_offset))
	}

	pub fn get_tid_offsets(&self, tid: &str) -> Option<(u64, u64)>
	{
		let entry = self.entries.get(tid)?;
		Some((entry.offset, entry.offset + entry.length))
	}
}
