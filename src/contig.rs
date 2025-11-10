use std::collections::HashMap;
use std::borrow::Cow;
use std::sync::Arc;

use tokio::io::SeekFrom;
use tokio::io::AsyncSeekExt;
use tokio::io::AsyncReadExt;
use tokio::sync::Mutex;

use async_trait::async_trait;

use crate::FaiIndex;
use crate::error;

#[async_trait]
pub trait Source
{
	async fn sequence(&self) -> Option<String>;
	async fn read_region(&self, start: u64, end: u64) -> Option<String>;
}

pub struct MemoryContig
{
	pub sequence: String,
}

#[async_trait]
impl Source for MemoryContig
{
	async fn sequence(&self) -> Option<String>
	{
		Some(self.sequence.clone())
	}

	async fn read_region(&self, start: u64, end: u64) -> Option<String>
	{
		if end as usize > self.sequence.len() || start > end
		{
			return None;
		}
		Some(self.sequence.get(start as usize..end as usize)?.to_string())
	}
}

pub struct FileContig<R>
{
	pub tid: String,
	pub index: Option<Arc<FaiIndex>>,
	pub reader: Arc<Mutex<R>>,
}

#[async_trait]
impl<R> Source for FileContig<R>
where
	R: tokio::io::AsyncRead + tokio::io::AsyncSeek + Unpin + Send,
{
	async fn sequence(&self) -> Option<String>
	{
		let (file_start, file_end) = self
			.index
			.as_ref()?
			.get_tid_offsets(&self.tid)
			.ok_or(error::Error::InvalidRegion)
			.ok()?;

		let mut reader = self.reader.lock().await;
		reader.seek(SeekFrom::Start(file_start)).await.ok()?;

		let mut buf = vec![0; (file_end - file_start) as usize];
		reader.read_exact(&mut buf).await.ok()?;

		Some(
			buf.into_iter()
				.filter(|&b| b != b'\n' && b != b'\r')
				.map(|b| b as char)
				.collect(),
		)
	}

	async fn read_region(&self, start: u64, end: u64) -> Option<String>
	{
		// compute offset via FAI line-length info
		let (file_start, file_end) = self
			.index
			.as_ref()?
			.get_region_offsets(&self.tid, start, end)?;
		let mut reader = self.reader.lock().await;
		reader.seek(SeekFrom::Start(file_start)).await.ok()?;

		let mut buf = vec![0; (file_end - file_start) as usize];
		reader.read_exact(&mut buf).await.ok()?;
		Some(
			buf.into_iter()
				.filter(|&b| b != b'\n' && b != b'\r')
				.map(|b| b as char)
				.collect(),
		)
	}
}

pub struct Contig
{
	pub tid: String,
	pub source: Box<dyn Source + Send + Sync>,
}

impl Contig
{
	pub async fn sequence(&mut self) -> Option<String>
	{
		self.source.sequence().await
	}

	pub async fn read_region(&mut self, start: u64, end: u64) -> Option<String>
	{
		self.source.read_region(start, end).await
	}
}

pub type Contigs = HashMap<Cow<'static, str>, Contig>;
