// Copyright 2024 - Strixpyrr
// SPDX-License-Identifier: Apache-2.0

use crate::{DataSink, Error, Result};

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

#[cfg(feature = "unstable_uninit_slice")]
use core::mem::MaybeUninit;

#[cfg(feature = "unstable_uninit_slice")]
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
