use thiserror::Error;

use mach::{
	exception_types::{
		exception_behavior_array_t, exception_behavior_t, exception_flavor_array_t,
		exception_mask_array_t, exception_mask_t, EXCEPTION_DEFAULT, EXC_MASK_ALL,
		MACH_EXCEPTION_CODES,
	},
	kern_return::{kern_return_t, KERN_SUCCESS},
	mach_port::{mach_port_allocate, mach_port_insert_right},
	mach_types::{exception_handler_array_t, exception_handler_t, task_t},
	message::{
		mach_msg, mach_msg_body_t, mach_msg_header_t, mach_msg_trailer_t, mach_msg_type_number_t,
		MACH_MSG_SUCCESS, MACH_MSG_TYPE_MAKE_SEND, MACH_RCV_LARGE, MACH_RCV_MSG, MACH_RCV_TIMEOUT,
		MACH_RCV_TOO_LARGE,
	},
	port::{mach_port_name_t, mach_port_t, MACH_PORT_NULL, MACH_PORT_RIGHT_RECEIVE},
	thread_status::{thread_state_flavor_t, THREAD_STATE_NONE},
	traps::mach_task_self,
	vm_types::natural_t,
};

use super::TaskPort;

// Sadly these are not defined in the mach crate for some reason.
// From https://github.com/apple/darwin-xnu/blob/master/osfmk/mach/task.defs
extern "C" {
	pub fn task_get_exception_ports(
		task: task_t,
		exception_mask: exception_mask_t,
		old_masks: exception_mask_array_t,
		old_masks_len: mach_msg_type_number_t,
		old_handlers: exception_handler_array_t,
		old_behaviors: exception_behavior_array_t,
		old_flavors: exception_flavor_array_t,
	);

	pub fn task_set_exception_ports(
		task: task_t,
		exception_mask: exception_mask_t,
		new_port: mach_port_t,
		behavior: exception_behavior_t,
		new_flavor: thread_state_flavor_t,
	) -> kern_return_t;

	pub fn task_swap_exception_ports(
		task: task_t,
		exception_mask: exception_mask_t,
		new_port: mach_port_t,
		behavior: exception_behavior_t,
		new_flavor: thread_state_flavor_t,
		old_masks: exception_mask_array_t,
		old_masks_len: *mut mach_msg_type_number_t,
		old_handlers: exception_handler_array_t,
		old_behaviors: exception_behavior_array_t,
		old_flavors: exception_flavor_array_t,
	) -> kern_return_t;
}

/*
From https://github.com/llvm/llvm-project/blob/62ec4ac90738a5f2d209ed28c822223e58aaaeb7/lldb/tools/debugserver/source/MacOSX/MachException.cpp#L397
#define PREV_EXC_MASK_ALL (EXC_MASK_BAD_ACCESS |                \
						 EXC_MASK_BAD_INSTRUCTION |             \
						 EXC_MASK_ARITHMETIC |                  \
						 EXC_MASK_EMULATION |                   \
						 EXC_MASK_SOFTWARE |                    \
						 EXC_MASK_BREAKPOINT |                  \
						 EXC_MASK_SYSCALL |                     \
						 EXC_MASK_MACH_SYSCALL |                \
						 EXC_MASK_RPC_ALERT |                   \
						 EXC_MASK_RESOURCE |                    \
						 EXC_MASK_GUARD |                       \
						 EXC_MASK_MACHINE)
*/

#[derive(Debug, Error)]
pub enum MachExceptionHandlerError {
	#[error("could not get task port from pid")]
	TaskPortError(std::io::Error),
	#[error("could not create exception port")]
	CreatePortError(std::io::Error),
	#[error("could not swap new and old exception handler configuration")]
	SwapExceptionError(std::io::Error),
}

// This is not defined in the mach crate either.
// From https://github.com/apple/darwin-xnu/blob/master/osfmk/mach/i386/exception.h
const EXC_TYPES_COUNT: usize = 14;

#[repr(C)]
#[derive(Debug)]
struct MessageBufferBody {
	pub info: mach_msg_body_t,
	pub data: [u8],
}
#[repr(C)]
#[derive(Debug)]
struct MessageBufferTrailer {
	pub info: mach_msg_trailer_t,
	pub data: [u8],
}

#[repr(C)]
struct MessageBuffer {
	// The data needs to be "naturally" aligned.
	buffer: Vec<natural_t>,
}
impl MessageBuffer {
	const ELEMENT_SIZE: usize = std::mem::size_of::<natural_t>();
	/// Minumum number of `natural_t`s that the buffer is initialized to.
	const MINIMUM_SIZE: usize = Self::natural_count(
		std::mem::size_of::<mach_msg_header_t>() + std::mem::size_of::<mach_msg_trailer_t>(),
	);

	const fn natural_count(byte_count: usize) -> usize {
		byte_count / Self::ELEMENT_SIZE + (byte_count % Self::ELEMENT_SIZE != 0) as usize
	}

	pub fn new() -> Self {
		MessageBuffer {
			buffer: vec![0; Self::MINIMUM_SIZE],
		}
	}

	/// Returns the size of the buffer in bytes.
	///
	/// This is including the header.
	pub fn size(&self) -> usize {
		self.buffer.len() * Self::ELEMENT_SIZE
	}

	/// Reserves space for at least `additional` more bytes.
	pub fn reserve(&mut self, additional: usize) {
		self.buffer.resize(
			Self::natural_count(self.buffer.len() * Self::ELEMENT_SIZE + additional),
			0,
		)
	}

	/// ## Safety
	/// * The data loaded into this buffer must be a valid header
	/// TODO: Can zeroed header be "valid" header? What even is documentation
	pub fn header(&self) -> &mach_msg_header_t {
		debug_assert!(self.buffer.len() >= Self::MINIMUM_SIZE);

		unsafe { &*(self.buffer.as_ptr() as *const mach_msg_header_t) }
	}

	pub unsafe fn header_mut(&mut self) -> &mut mach_msg_header_t {
		debug_assert!(self.buffer.len() >= Self::MINIMUM_SIZE);

		// Safe because it is correctly aligned and has enough space
		unsafe { &mut *(self.buffer.as_mut_ptr() as *mut mach_msg_header_t) }
	}

	/// Returns the bytes covering the body of the message according to the header.
	pub fn body(&self) -> Option<&MessageBufferBody> {
		const HEADER_SIZE: usize = std::mem::size_of::<mach_msg_header_t>();

		if self.header().msgh_size as usize <= HEADER_SIZE {
			return None;
		}
		assert!(self.header().msgh_size as usize <= self.size());

		let start = unsafe { (self.buffer.as_ptr() as *const u8).add(HEADER_SIZE) };
		let size = self.header().msgh_size as usize - HEADER_SIZE;

		let rf = unsafe {
			let ptr = std::ptr::slice_from_raw_parts(start, size) as *const MessageBufferBody;

			&*ptr
		};

		Some(rf)
	}

	pub fn trailer(&self) -> Option<&MessageBufferTrailer> {
		if self.header().msgh_size as usize == self.size() {
			return None;
		}
		assert!(
			self.header().msgh_size as usize + std::mem::size_of::<mach_msg_trailer_t>()
				<= self.size()
		);

		let start =
			unsafe { (self.buffer.as_ptr() as *const u8).add(self.header().msgh_size as usize) };
		let size = unsafe { (*(start as *const mach_msg_trailer_t)).msgh_trailer_size as usize };

		let rf = unsafe {
			let ptr = std::ptr::slice_from_raw_parts(start, size) as *const MessageBufferTrailer;

			&*ptr
		};

		Some(rf)
	}
}
impl std::fmt::Debug for MessageBuffer {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		f.debug_struct("MessageBuffer")
			.field("header", self.header())
			.field("body", &self.body())
			.field("trailer", &self.trailer())
			.finish()
	}
}

#[derive(Debug)]
pub struct MachExceptionHandler {
	saved_length: mach_msg_type_number_t,
	saved_masks: [exception_mask_t; EXC_TYPES_COUNT],
	saved_handlers: [exception_handler_t; EXC_TYPES_COUNT],
	saved_behaviors: [exception_behavior_t; EXC_TYPES_COUNT],
	saved_flavors: [thread_state_flavor_t; EXC_TYPES_COUNT],

	task_port: TaskPort,
	exception_port: TaskPort,

	buffer: MessageBuffer,
}
impl MachExceptionHandler {
	pub fn new(pid: libc::pid_t) -> Result<Self, MachExceptionHandlerError> {
		let task_port = TaskPort::new(pid).map_err(MachExceptionHandlerError::TaskPortError)?;
		let exception_port = unsafe {
			Self::create_exception_port().map_err(MachExceptionHandlerError::CreatePortError)?
		};

		let mut me = MachExceptionHandler {
			saved_length: 0,
			saved_masks: Default::default(),
			saved_handlers: Default::default(),
			saved_behaviors: Default::default(),
			saved_flavors: Default::default(),

			task_port,
			exception_port,

			buffer: MessageBuffer::new(),
		};
		unsafe {
			me.swap_exception_ports()
				.map_err(MachExceptionHandlerError::SwapExceptionError)?;
		}

		Ok(me)
	}

	unsafe fn create_exception_port() -> Result<TaskPort, std::io::Error> {
		let mut exception_port: mach_port_t = Default::default();

		// create a new exception port
		let self_port = mach_task_self();
		let result = mach_port_allocate(
			self_port,
			MACH_PORT_RIGHT_RECEIVE,
			&mut exception_port as *mut mach_port_name_t,
		);
		if result != KERN_SUCCESS {
			return Err(std::io::Error::last_os_error());
		}

		let result = mach_port_insert_right(
			self_port,
			exception_port,
			exception_port,
			MACH_MSG_TYPE_MAKE_SEND,
		);
		if result != KERN_SUCCESS {
			return Err(std::io::Error::last_os_error());
		}

		Ok(TaskPort::from_raw(exception_port))
	}

	unsafe fn swap_exception_ports(&mut self) -> Result<(), std::io::Error> {
		// swap the exception port for the process
		let result = task_swap_exception_ports(
			self.task_port.get(),
			EXC_MASK_ALL,
			self.exception_port.get(),
			(EXCEPTION_DEFAULT | MACH_EXCEPTION_CODES) as exception_behavior_t,
			THREAD_STATE_NONE,
			self.saved_masks.as_mut_ptr(),
			&mut self.saved_length,
			self.saved_handlers.as_mut_ptr(),
			self.saved_behaviors.as_mut_ptr(),
			self.saved_flavors.as_mut_ptr(),
		);
		if result != KERN_SUCCESS {
			return Err(std::io::Error::last_os_error());
		}

		Ok(())
	}

	unsafe fn restore_exception_ports(&mut self) -> Result<(), std::io::Error> {
		let len = self.saved_length as usize;
		self.saved_length = 0;

		for i in 0..len {
			let result = task_set_exception_ports(
				self.task_port.get(),
				self.saved_masks[i],
				self.saved_handlers[i],
				self.saved_behaviors[i],
				self.saved_flavors[i],
			);

			if result != KERN_SUCCESS {
				return Err(std::io::Error::last_os_error());
			}
		}

		Ok(())
	}

	/// Attempts to receive a message.
	///
	/// This method does not block to wait for a message.
	pub fn try_receive(&mut self) -> Option<usize> {
		loop {
			let result = unsafe {
				mach_msg(
					self.buffer.header_mut(),
					MACH_RCV_MSG | MACH_RCV_TIMEOUT | MACH_RCV_LARGE,
					0,
					self.buffer.size() as u32,
					self.exception_port.get(),
					0, // in ms
					MACH_PORT_NULL,
				)
			};

			if result == MACH_RCV_TOO_LARGE {
				self.buffer.reserve(4);

				continue;
			}
			if result != MACH_MSG_SUCCESS {
				break None;
			}

			eprintln!("buffer: {:?}", self.buffer);
			break Some(0);
		}
	}
}
impl Drop for MachExceptionHandler {
	fn drop(&mut self) {
		let result = unsafe { self.restore_exception_ports() };

		debug_assert!(result.is_ok());
	}
}
