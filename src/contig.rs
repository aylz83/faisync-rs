use std::collections::HashMap;
use std::borrow::Cow;
use std::sync::Arc;

use tokio::io::SeekFrom;
use tokio::io::AsyncSeekExt;
use tokio::io::AsyncReadExt;
use tokio::sync::Mutex;

use crate::FaiIndex;
use crate::error;

pub trait SourceSync
{
	fn base_at(&self, pos: u64) -> Option<u8>;
	fn sequence(&self) -> Option<String>;
	fn read_region(&self, start: u64, end: u64) -> Option<String>;
}

#[async_trait::async_trait]
pub trait SourceAsync
{
	async fn base_at(&self, pos: u64) -> Option<u8>;
	async fn sequence(&self) -> Option<String>;
	async fn read_region(&self, start: u64, end: u64) -> Option<String>;
}
pub struct SyncSourceAdapter<S>(pub S);

#[async_trait::async_trait]
impl<S> SourceAsync for SyncSourceAdapter<S>
where
	S: SourceSync + Send + Sync,
{
	async fn base_at(&self, pos: u64) -> Option<u8>
	{
		self.0.base_at(pos)
	}

	async fn sequence(&self) -> Option<String>
	{
		self.0.sequence()
	}

	async fn read_region(&self, start: u64, end: u64) -> Option<String>
	{
		self.0.read_region(start, end)
	}
}

pub struct MemoryContig
{
	pub sequence: String,
}

impl SourceSync for MemoryContig
{
	fn base_at(&self, pos: u64) -> Option<u8>
	{
		self.sequence.as_bytes().get(pos as usize).copied()
	}

	fn sequence(&self) -> Option<String>
	{
		Some(self.sequence.clone())
	}

	fn read_region(&self, start: u64, end: u64) -> Option<String>
	{
		if end as usize > self.sequence.len() || start > end
		{
			return None;
		}
		self.sequence
			.get(start as usize..end as usize)
			.map(str::to_owned)
	}
}

pub struct FileContig<R>
{
	pub tid: String,
	pub index: Option<Arc<FaiIndex>>,
	pub reader: Arc<Mutex<R>>,
}

#[async_trait::async_trait]
impl<R> SourceAsync for FileContig<R>
where
	R: tokio::io::AsyncRead + tokio::io::AsyncSeek + Unpin + Send,
{
	async fn base_at(&self, pos: u64) -> Option<u8>
	{
		let (file_pos, _) = self
			.index
			.as_ref()?
			.get_region_offsets(&self.tid, pos, pos + 1)?;

		let mut reader = self.reader.lock().await;
		reader.seek(SeekFrom::Start(file_pos)).await.ok()?;

		let mut byte = [0u8; 1];
		reader.read_exact(&mut byte).await.ok()?;

		match byte[0]
		{
			b'\n' | b'\r' => None,
			b => Some(b.to_ascii_uppercase()),
		}
	}

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
	pub source: Box<dyn SourceAsync + Send + Sync>,
}

impl Contig
{
	pub async fn base_at(&self, start: u64) -> Option<u8>
	{
		self.source.base_at(start).await
	}

	pub async fn sequence(&self) -> Option<String>
	{
		self.source.sequence().await
	}

	pub async fn read_region(&self, start: u64, end: u64) -> Option<String>
	{
		self.source.read_region(start, end).await
	}
}

pub type Contigs = HashMap<Cow<'static, str>, Contig>;
