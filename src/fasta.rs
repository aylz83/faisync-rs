use tokio::fs::File;
use tokio::io::{SeekFrom, AsyncSeekExt, AsyncReadExt, BufReader};

use std::path::Path;

use crate::error;
use crate::fai::FaiIndex;
use crate::contig::Contig;

pub struct Fasta
{
	reader: BufReader<File>,
	index: Option<FaiIndex>,
}

impl Fasta
{
	pub async fn from_path<P>(fasta_path: P, fai_path: Option<P>) -> error::Result<Self>
	where
		P: AsRef<Path>,
	{
		let fasta_path = fasta_path.as_ref();

		let fai_index = if let Some(fai_path) = fai_path
		{
			Some(FaiIndex::from_path(fai_path).await?)
		}
		else
		{
			let fai_path = fasta_path.with_extension(
				fasta_path
					.extension()
					.map(|ext| format!("{}.fai", ext.to_string_lossy()))
					.unwrap_or_else(|| "fai".to_string()),
			);

			if tokio::fs::metadata(&fai_path).await.is_ok()
			{
				Some(FaiIndex::from_path(fai_path).await?)
			}
			else
			{
				None
			}
		};

		let file = File::open(fasta_path).await?;
		let reader = BufReader::new(file);

		Ok(Fasta {
			reader,
			index: fai_index,
		})
	}

	pub async fn read_region(&mut self, tid: &str, start: u64, end: u64) -> error::Result<String>
	{
		let (file_start, file_end) = self
			.index
			.as_mut()
			.ok_or_else(|| error::Error::NoFAIDX)?
			.get_region_offsets(tid, start, end)
			.ok_or_else(|| error::Error::InvalidRegion)?;

		self.reader
			.seek(tokio::io::SeekFrom::Start(file_start))
			.await?;

		let mut buf = vec![0u8; (file_end - file_start) as usize];
		self.reader.read_exact(&mut buf).await?;

		Ok(buf
			.into_iter()
			.filter(|&b| b != b'\n' && b != b'\r')
			.map(|b| b as char)
			.collect())
	}

	pub async fn read_tid(&mut self, tid: &str) -> error::Result<Contig>
	{
		let (file_start, file_end) = self
			.index
			.as_ref()
			.ok_or(error::Error::NoFAIDX)?
			.get_tid_offsets(tid)
			.ok_or(error::Error::InvalidRegion)?;

		self.reader.seek(SeekFrom::Start(file_start)).await?;

		let mut buf = vec![0u8; (file_end - file_start) as usize];
		self.reader.read_exact(&mut buf).await?;

		let seq: String = buf
			.into_iter()
			.filter(|&b| b != b'\n' && b != b'\r')
			.map(|b| b as char)
			.collect();

		Ok(Contig {
			tid: tid.to_string(),
			sequence: seq,
		})
	}
}

pub trait ReverseComplement
{
	fn reverse_complement(&self) -> String;
}

impl ReverseComplement for String
{
	fn reverse_complement(&self) -> String
	{
		self.chars()
			.rev()
			.map(|c| match c
			{
				'A' | 'a' => 'T',
				'T' | 't' => 'A',
				'C' | 'c' => 'G',
				'G' | 'g' => 'C',
				'N' | 'n' => 'N',
				_ => 'N',
			})
			.collect()
	}
}

impl ReverseComplement for &str
{
	fn reverse_complement(&self) -> String
	{
		self.chars()
			.rev()
			.map(|c| match c
			{
				'A' | 'a' => 'T',
				'T' | 't' => 'A',
				'C' | 'c' => 'G',
				'G' | 'g' => 'C',
				'N' | 'n' => 'N',
				_ => 'N',
			})
			.collect()
	}
}

#[cfg(test)]
mod tests
{
	use super::*;
	use tempfile::tempdir;
	use tokio::io::AsyncWriteExt;

	#[test]
	fn test_reverse_complement_string()
	{
		let seq = "ATGCN".to_string();
		let rev = seq.reverse_complement();
		assert_eq!(rev, "NGCAT"); // reversed and complemented
	}

	#[test]
	fn test_reverse_complement_str()
	{
		let seq = "atgcn";
		let rev = seq.reverse_complement();
		assert_eq!(rev, "NGCAT"); // case-insensitive mapping
	}

	#[test]
	fn test_reverse_complement_unknown()
	{
		let seq = "ATGXZ".to_string();
		let rev = seq.reverse_complement();
		assert_eq!(rev, "NNCAT"); // unknown characters mapped to 'N'
	}

	#[test]
	fn test_empty_sequence()
	{
		let seq = "".to_string();
		let rev = seq.reverse_complement();
		assert_eq!(rev, "");
	}

	async fn create_test_fasta_and_fai() -> (tempfile::TempDir, String, String)
	{
		let dir = tempdir().unwrap();
		let fasta_path = dir.path().join("test.fasta");
		let fai_path = dir.path().join("test.fasta.fai");

		// Write FASTA
		let mut fasta_file = File::create(&fasta_path).await.unwrap();
		fasta_file
			.write_all(b">chr1\nACGTACGTACGT\n")
			.await
			.unwrap();
		fasta_file.flush().await.unwrap();

		// Write FAI - name length offset line_bases line_width
		let mut fai_file = File::create(&fai_path).await.unwrap();
		fai_file.write_all(b"chr1\t12\t6\t12\t13\n").await.unwrap();
		fai_file.flush().await.unwrap();

		(
			dir,
			fasta_path.to_string_lossy().to_string(),
			fai_path.to_string_lossy().to_string(),
		)
	}

	#[tokio::test]
	async fn test_fasta_from_path_with_fai()
	{
		let (_dir, fasta_path, fai_path) = create_test_fasta_and_fai().await;

		let fasta = Fasta::from_path(fasta_path.clone(), Some(fai_path.clone()))
			.await
			.unwrap();

		assert!(fasta.index.is_some());
	}

	#[tokio::test]
	async fn test_fasta_from_path_auto_detect_fai()
	{
		let (_dir, fasta_path, _fai_path) = create_test_fasta_and_fai().await;

		let fasta = Fasta::from_path(fasta_path.clone(), None).await.unwrap();

		assert!(fasta.index.is_some());
	}

	#[tokio::test]
	async fn test_read_region_forward()
	{
		let (_dir, fasta_path, fai_path) = create_test_fasta_and_fai().await;

		let mut fasta = Fasta::from_path(fasta_path, Some(fai_path)).await.unwrap();

		let seq = fasta.read_region("chr1", 0, 4).await.unwrap();
		assert_eq!(seq, "ACGT");
	}

	#[tokio::test]
	async fn test_read_region_invalid_region()
	{
		let (_dir, fasta_path, fai_path) = create_test_fasta_and_fai().await;

		let mut fasta = Fasta::from_path(fasta_path, Some(fai_path)).await.unwrap();

		let result = fasta.read_region("chr1", 100, 200).await;
		assert!(matches!(result.unwrap_err(), error::Error::InvalidRegion));
	}

	#[tokio::test]
	async fn test_read_region_no_fai()
	{
		let dir = tempdir().unwrap();
		let fasta_path = dir.path().join("test.fasta");

		let mut fasta_file = File::create(&fasta_path).await.unwrap();
		fasta_file
			.write_all(b">chr1\nACGTACGTACGT\n")
			.await
			.unwrap();
		fasta_file.flush().await.unwrap();

		let mut fasta = Fasta::from_path(&fasta_path, None).await.unwrap();

		let result = fasta.read_region("chr1", 0, 4).await;
		assert!(matches!(result.unwrap_err(), error::Error::NoFAIDX));
	}

	#[tokio::test]
	async fn test_read_tid_and_region_in_memory()
	{
		let (_dir, fasta_path, fai_path) = create_test_fasta_and_fai().await;
		let mut fasta = Fasta::from_path(&fasta_path, Some(&fai_path))
			.await
			.unwrap();

		let contig = fasta.read_tid("chr1").await.unwrap();
		assert_eq!(contig.tid, "chr1");
		assert_eq!(contig.sequence, "ACGTACGTACGT");

		let region = contig.read_region(4, 8).unwrap();
		assert_eq!(region, "ACGT");
	}
}
