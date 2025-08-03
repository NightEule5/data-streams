// Copyright 2024 - Strixpyrr
// SPDX-License-Identifier: Apache-2.0

#![cfg(all(feature = "std", feature = "alloc", feature = "utf8"))]

use std::collections::VecDeque;
use std::fmt::{Debug, Formatter};
use std::str::from_utf8;
use proptest::prelude::*;
use proptest::string::bytes_regex;
use data_streams::DataSource;

struct Utf8Deque {
	deque: VecDeque<u8>,
	value: String
}

impl Debug for Utf8Deque {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Utf8Deque")
			.field("deque", &self.deque.as_slices())
			.field("value", &self.value)
			.finish()
	}
}

fn utf8_deque() -> impl Strategy<Value = Utf8Deque> {
	(any::<String>(), any::<usize>()).prop_map(|(value, mut rotation)| {
		let mut deque = VecDeque::with_capacity(value.len());
		rotation %= deque.len() + 1;
		let (front, back) = value.as_bytes().split_at(rotation);
		deque.extend(front);
		deque.rotate_left(rotation);
		deque.extend(back);
		Utf8Deque { deque, value }
	})
}

struct InvalidUtf8Deque {
	deque: VecDeque<u8>,
}

impl Debug for InvalidUtf8Deque {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("InvalidUtf8Deque")
			.field("deque", &self.deque.as_slices())
			.finish()
	}
}

fn invalid_deque() -> impl Strategy<Value = InvalidUtf8Deque> {
	(bytes_regex("(?s-u:.*)").unwrap(), any::<usize>()).prop_filter(
		"only invalid UTF-8",
		|(bytes, _)| from_utf8(bytes).is_err()
	).prop_map(|(bytes, mut rotation)| {
		let mut deque = VecDeque::with_capacity(bytes.len());
		rotation %= deque.len() + 1;
		let (front, back) = bytes.split_at(rotation);
		deque.extend(front);
		deque.rotate_left(rotation);
		deque.extend(back);
		InvalidUtf8Deque { deque }
	})
}

proptest! {
	#[test]
	fn slice_nominal(input in "(?s:.*)") {
		let mut buf = vec![0; input.len()];
		let result = input.as_bytes().read_utf8(&mut buf);
		prop_assert!(result.is_ok());
		prop_assert_eq!(result.unwrap(), input);
	}
	
	#[test]
	fn vec_nominal(input in "(?s:.*)") {
		let mut buf = vec![0; input.len()];
		let result = input.clone().into_bytes().read_utf8(&mut buf);
		prop_assert!(result.is_ok());
		prop_assert_eq!(result.unwrap(), input);
	}
	
	#[test]
	fn vec_deque_nominal(input in utf8_deque()) {
		let mut buf = vec![0; input.value.len()];
		let result = input.deque.clone().read_utf8(&mut buf);
		prop_assert!(result.is_ok());
		prop_assert_eq!(result.unwrap(), input.value);
	}
	
	#[test]
	fn slice_invalid(input in bytes_regex("(?s-u:.*)").unwrap().prop_filter("only invalid UTF-8", |bytes|
		from_utf8(bytes).is_err()
	)) {
		let mut buf = vec![0; input.len()];
		let result = input.as_slice().read_utf8(&mut buf);
		prop_assert!(result.is_err());
	}
	
	#[test]
	fn vec_invalid(input in bytes_regex("(?s-u:.*)").unwrap().prop_filter("only invalid UTF-8", |bytes|
		from_utf8(bytes).is_err()
	)) {
		let mut buf = vec![0; input.len()];
		let result = input.clone().read_utf8(&mut buf);
		prop_assert!(result.is_err());
	}
	
	#[test]
	fn vec_deque_invalid(input in invalid_deque()) {
		let mut buf = vec![0; input.deque.len()];
		let result = input.deque.clone().read_utf8(&mut buf);
		prop_assert!(result.is_err());
	}
}
