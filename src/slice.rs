// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "alloc")]
use alloc::string::String;
#[cfg(feature = "alloc")]
use simdutf8::compat::from_utf8;
use crate::{
	Error,
	DataSink,
	Result,
	BufferAccess,
	DataSource,
};

impl DataSource for &[u8] {
	fn available(&self) -> usize { self.len() }
	fn request(&mut self, count: usize) -> Result<bool> {
		Ok(self.len() >= count)
	}

	fn skip(&mut self, mut count: usize) -> Result<usize> {
		count = count.min(self.len());
		self.drain_buffer(count);
		Ok(count)
	}
	
	fn read_bytes<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a [u8]> {
		Ok(read_bytes_infallible(self, buf))
	}

	#[cfg(feature = "alloc")]
	fn read_utf8<'a>(&mut self, mut count: usize, buf: &'a mut String) -> Result<&'a str> {
		count = count.min(self.len());
		let result = from_utf8(&self[..count]);
		*self = &self[count..];
		let start = buf.len();
		buf.push_str(result?);
		Ok(&buf[start..])
	}

	#[cfg(feature = "alloc")]
	fn read_utf8_to_end<'a>(&mut self, buf: &'a mut String) -> Result<&'a str> {
		self.read_utf8(self.len(), buf)
	}
}

impl BufferAccess for &[u8] {
	fn buffer_capacity(&self) -> usize { self.len() }
	fn buffer_count(&self) -> usize { self.len() }
	fn buffer(&self) -> &[u8] { self }
	fn fill_buffer(&mut self) -> Result<&[u8]> { Ok(self) }

	fn clear_buffer(&mut self) {
		*self = &[];
	}

	fn drain_buffer(&mut self, count: usize) {
		*self = &self[count..];
	}
}

impl DataSink for &mut [u8] {
	fn write_bytes(&mut self, buf: &[u8]) -> Result {
		mut_slice_write_bytes(self, buf, <[u8]>::copy_from_slice)
	}

	fn write_u8(&mut self, value: u8) -> Result {
		use core::convert::identity;
		mut_slice_push_u8(self, value, identity)
	}

	fn write_i8(&mut self, value: i8) -> Result {
		self.write_u8(value as u8)
	}
}

#[cfg(feature = "nightly_uninit_slice")]
use core::mem::MaybeUninit;

#[cfg(feature = "nightly_uninit_slice")]
impl DataSink for &mut [MaybeUninit<u8>] {
	fn write_bytes(&mut self, buf: &[u8]) -> Result {
		mut_slice_write_bytes(self, buf, |t, s| { MaybeUninit::copy_from_slice(t, s); })
	}

	fn write_u8(&mut self, value: u8) -> Result {
		mut_slice_push_u8(self, value, MaybeUninit::new)
	}

	fn write_i8(&mut self, value: i8) -> Result {
		self.write_u8(value as u8)
	}
}

use core::mem::take;

#[allow(clippy::mut_mut)]
fn mut_slice_write_bytes<T>(
	sink: &mut &mut [T],
	buf: &[u8],
	copy_from_slice: impl FnOnce(&mut [T], &[u8])
) -> Result {
	let len = buf.len().min(sink.len());
	// From <[_]>::take_mut
	let (target, empty) = take(sink).split_at_mut(len);
	*sink = empty;
	copy_from_slice(target, &buf[..len]);
	let remaining = buf.len() - len;
	if remaining > 0 {
		Err(Error::overflow(remaining))
	} else {
		Ok(())
	}
}

#[allow(clippy::mut_mut)]
fn mut_slice_push_u8<T>(
	sink: &mut &mut [T],
	value: u8,
	map: impl FnOnce(u8) -> T
) -> Result {
	if sink.is_empty() {
		Err(Error::overflow(1))
	} else {
		sink[0] = map(value);
		*sink = &mut take(sink)[1..];
		Ok(())
	}
}

pub(crate) fn read_bytes_infallible<'a>(source: &mut &[u8], sink: &'a mut [u8]) -> &'a [u8] {
	let len = source.len().min(sink.len());
	let (filled, unfilled) = sink.split_at_mut(len);
	filled.copy_from_slice(&source[..len]);
	*source = &source[len..];
	unfilled
}