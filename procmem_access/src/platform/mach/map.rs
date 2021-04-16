use thiserror::Error;

use mach::{
	kern_return::KERN_SUCCESS,
	mach_port::mach_port_deallocate,
	port::{MACH_PORT_NULL, mach_port_t},
	vm_prot::{VM_PROT_EXECUTE, VM_PROT_READ, VM_PROT_WRITE},
	vm_region::{VM_REGION_BASIC_INFO_64, vm_region_basic_info_64, vm_region_info_t},
	vm_types::{mach_vm_address_t, mach_vm_size_t}
};

use crate::{
	common::OffsetType,
	memory::map::{MemoryMap, MemoryPage, MemoryPagePermissions, MemoryPageType}
};

#[derive(Debug, Error)]
pub enum MachMemoryMapError {
	#[error("could not retrieve port handle")]
	PortError(std::io::Error)
}

pub struct MachMemoryMap {
	pages: Vec<MemoryPage>,
}
impl MachMemoryMap {
	pub fn new(pid: libc::pid_t) -> Result<Self, MachMemoryMapError> {
		let port = super::TaskPort::new(pid).map_err(MachMemoryMapError::PortError)?;
		let mut pages = Vec::new();

		let mut previous_address = 0;
		while let Some(page) = Self::enumerate_next_page(port.get(), previous_address) {
			previous_address = page.address_range[1].get();
			pages.push(page);
		}

		Ok(
			MachMemoryMap {
				pages
			}
		)
	}

	fn enumerate_next_page(
		port: mach_port_t,
		previous_address: mach_vm_address_t,
	) -> Option<MemoryPage> {
		let mut address = previous_address;
		let mut size: mach_vm_size_t = 0;
		let mut info: vm_region_basic_info_64 = Default::default();
		let mut info_count = vm_region_basic_info_64::count();
		let mut object_name: mach_port_t = Default::default();

		// TODO: I could not find any documentation, so this code is
		// just a best-effort guess, I don't really know how unsafe it is
		let res = unsafe {
			mach::vm::mach_vm_region(
				port,
				&mut address as *mut mach_vm_address_t,
				&mut size as *mut mach_vm_size_t,
				VM_REGION_BASIC_INFO_64,
				&mut info as *mut vm_region_basic_info_64 as vm_region_info_t,
				&mut info_count,
				&mut object_name
			)
		};

		if object_name != MACH_PORT_NULL {
			// TODO: Documentation would probably reveal what this is and why it is necessary
			// until then, I'll just believe the internet
			unsafe {
				let res = mach_port_deallocate(port, object_name);
				debug_assert_eq!(res, KERN_SUCCESS);
			}
		}
		if res != KERN_SUCCESS {
			return None;
		}

		let page = MemoryPage {
			address_range: [
				OffsetType::new(address).unwrap(),
				OffsetType::new(address + size).unwrap()
			],
			permissions: MemoryPagePermissions::new(
				info.protection & VM_PROT_READ != 0,
				info.protection & VM_PROT_WRITE != 0,
				info.protection & VM_PROT_EXECUTE != 0,
				info.shared != 0
			),
			offset: info.offset,
			// TODO: This info can probably be retrieved from somewhere, maybe `object_name`?
			page_type: MemoryPageType::Unknown
		};
		
		Some(page)
	}
}
impl MemoryMap for MachMemoryMap {
	fn pages(&self) -> &[MemoryPage] {
		&self.pages
	}
}