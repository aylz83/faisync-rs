use nom::{bytes::complete::take_until, character::complete::{digit1, line_ending}, combinator::map_res, IResult};
use nom::Parser;

use crate::fai::FaiEntry;

pub(crate) fn parse_u64(input: &str) -> IResult<&str, u64>
{
	map_res(digit1, str::parse).parse(input)
}

pub(crate) fn parse_fai_line(input: &str) -> IResult<&str, FaiEntry>
{
	let (input, (name, _, length, _, offset, _, line_bases, _, line_width, _)) = ((
		take_until("\t"), // name
		nom::character::complete::char('\t'),
		parse_u64,
		nom::character::complete::char('\t'),
		parse_u64,
		nom::character::complete::char('\t'),
		parse_u64,
		nom::character::complete::char('\t'),
		parse_u64,
		line_ending,
	))
		.parse(input)?;

	Ok((
		input,
		FaiEntry {
			name: name.to_string(),
			length,
			offset,
			line_bases,
			line_width,
		},
	))
}
