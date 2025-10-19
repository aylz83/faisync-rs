pub struct Contig
{
	pub tid: String,
	pub sequence: String,
}

impl Contig
{
	pub fn read_region(&self, start: u64, end: u64) -> Option<String>
	{
		if end as usize > self.sequence.len() || start > end
		{
			return None;
		}
		Some(self.sequence.get(start as usize..end as usize)?.to_string())
	}
}
