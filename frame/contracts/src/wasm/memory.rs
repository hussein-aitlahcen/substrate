use sp_runtime::DispatchError;
use sp_sandbox::default_executor::Memory;
use sp_sandbox::SandboxMemory;
use sp_std::vec::Vec;
use sp_std::vec;

type MemoryResult<T> = Result<T, DispatchError>;

/// Describes some data allocated in Wasm's linear memory.
/// A pointer to an instance of this can be returned over FFI boundaries.
///
/// This is the same as `cosmwasm_std::memory::Region`
/// but defined here to allow Wasmer specific implementation.
#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
struct Region {
	/// The beginning of the region expressed as bytes from the beginning of the linear memory
	pub offset: u32,
	/// The number of bytes available in this region
	pub capacity: u32,
	/// The number of bytes used in this region
	pub length: u32,
}

/// Expects a (fixed size) Region struct at ptr, which is read. This links to the
/// memory region, which is copied in the second step.
/// Errors if the length of the region exceeds `max_length`.
pub fn read_region(memory: &Memory, ptr: u32, max_length: usize) -> MemoryResult<Vec<u8>> {
	let region = get_region(memory, ptr)?;
	if region.length > max_length as u32 {
		return Err(DispatchError::Other("region too big"));
	}
	let mut data = vec![0u8; region.length as usize];
	memory
		.get(region.offset, &mut data)
		.map_err(|_| DispatchError::Other("couldn't extract region"))?;
	Ok(data)
}

/// A prepared and sufficiently large memory Region is expected at ptr that points to pre-allocated memory.
///
/// Returns number of bytes written on success.
pub fn write_region(memory: &Memory, ptr: u32, data: &[u8]) -> MemoryResult<()> {
	let mut region = get_region(memory, ptr)?;
	let region_capacity = region.capacity as usize;
	if data.len() > region_capacity {
		return Err(DispatchError::Other("memory region too small"));
	}
	memory
		.set(region.offset, data)
		.map_err(|_| DispatchError::Other("couldn't extract region"))?;
	region.length = data.len() as u32;
    set_region(memory, ptr, region)?;
	Ok(())
}

// pub fn typed_write_region<T>(memory: &Memory, ptr: u32, data: &T) -> MemoryResult<()> {
// 	let mut region = get_region(memory, ptr)?;
// 	let region_capacity = region.capacity as usize;
// 	if data.len() > region_capacity {
// 		return Err(DispatchError::Other("memory region too small"));
// 	}
// 	memory
// 		.set(region.offset, data)
// 		.map_err(|_| DispatchError::Other("couldn't extract region"))?;
// 	region.length = data.len() as u32;
//     set_region(memory, ptr, region)?;
// 	Ok(())
// }

/// Reads in a Region at ptr in wasm memory and returns a copy of it
fn get_region(memory: &Memory, ptr: u32) -> MemoryResult<Region> {
	memory
		.typed_get::<Region>(ptr)
		.map_err(|_| DispatchError::Other("cannot read region"))
}

/// Performs plausibility checks in the given Region. Regions are always created by the
/// contract and this can be used to detect problems in the standard library of the contract.
fn validate_region(region: &Region) -> MemoryResult<()> {
	if region.offset == 0 {
		return Err(DispatchError::Other("zero region"));
	}
	if region.length > region.capacity {
		return Err(DispatchError::Other("region length exceeds capacity"));
	}
	if region.capacity > (u32::MAX - region.offset) {
		return Err(DispatchError::Other("region out of range"));
	}
	Ok(())
}

/// Overrides a Region at ptr in wasm memory with data
fn set_region(memory: &Memory, ptr: u32, data: Region) -> MemoryResult<()> {
	memory.typed_set(ptr, &data)
		.map_err(|_| DispatchError::Other("cannot read region"))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn validate_region_passes_for_valid_region() {
		// empty
		let region = Region { offset: 23, capacity: 500, length: 0 };
		validate_region(&region).unwrap();

		// half full
		let region = Region { offset: 23, capacity: 500, length: 250 };
		validate_region(&region).unwrap();

		// full
		let region = Region { offset: 23, capacity: 500, length: 500 };
		validate_region(&region).unwrap();

		// at end of linear memory (1)
		let region = Region { offset: u32::MAX, capacity: 0, length: 0 };
		validate_region(&region).unwrap();

		// at end of linear memory (2)
		let region = Region { offset: 1, capacity: u32::MAX - 1, length: 0 };
		validate_region(&region).unwrap();
	}

	// #[test]
	// fn validate_region_fails_for_zero_offset() {
	// 	let region = Region { offset: 0, capacity: 500, length: 250 };
	// 	let result = validate_region(&region);
	// 	match result.unwrap_err() {
	// 		RegionValidationError::ZeroOffset { .. } => {},
	// 		e => panic!("Got unexpected error: {:?}", e),
	// 	}
	// }

	// #[test]
	// fn validate_region_fails_for_length_exceeding_capacity() {
	// 	let region = Region { offset: 23, capacity: 500, length: 501 };
	// 	let result = validate_region(&region);
	// 	match result.unwrap_err() {
	// 		RegionValidationError::LengthExceedsCapacity { length, capacity, .. } => {
	// 			assert_eq!(length, 501);
	// 			assert_eq!(capacity, 500);
	// 		},
	// 		e => panic!("Got unexpected error: {:?}", e),
	// 	}
	// }

	// #[test]
	// fn validate_region_fails_when_exceeding_address_space() {
	// 	let region = Region { offset: 23, capacity: u32::MAX, length: 501 };
	// 	let result = validate_region(&region);
	// 	match result.unwrap_err() {
	// 		RegionValidationError::OutOfRange { offset, capacity, .. } => {
	// 			assert_eq!(offset, 23);
	// 			assert_eq!(capacity, u32::MAX);
	// 		},
	// 		e => panic!("Got unexpected error: {:?}", e),
	// 	}

	// 	let region = Region { offset: u32::MAX, capacity: 1, length: 0 };
	// 	let result = validate_region(&region);
	// 	match result.unwrap_err() {
	// 		RegionValidationError::OutOfRange { offset, capacity, .. } => {
	// 			assert_eq!(offset, u32::MAX);
	// 			assert_eq!(capacity, 1);
	// 		},
	// 		e => panic!("Got unexpected error: {:?}", e),
	// 	}
	// }
}
