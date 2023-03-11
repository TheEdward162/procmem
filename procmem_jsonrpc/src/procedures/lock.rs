//! ## Locking
//!
//! ### Create lock
//!
//! Method: `lock.create`
//! Params: `pid`, `locked`
//! Result: none
//! Error: `CreateLockError`, `LockError`
//!
//! Creates a new lock for the given `pid`. Parameter `locked` controls whether to attempt locking before returning.
//!
//! ### Lock
//!
//! Method: `lock.lock`
//! Params: `pid`
//! Result: `newly_locked`
//! Error: `LockError`, `NoSuchLockError`
//!
//! Locks an existing lock.
//!
//! ### Lock exclusive
//!
//! Method: `lock.lock_exclusive`
//! Params: `pid`
//! Result: none
//! Error: `ExclusiveLockError`, `NoSuchLockError`
//!
//! Locks an existing lock exclusively.
//!
//! ### Unlock
//!
//! Method: `lock.unlock`
//! Params: `pid`
//! Result: `released`
//! Error: `UnlockError`, `NotLockedError`, `NoSuchLockError`
//!
//! Unlock an existing, locked lock.
//!
//! ### Drop lock
//!
//! Method: `lock.drop`
//! Params: `pid`
//! Result: none
//! Error: `DropLockError`, `UnlockError`, `NoSuchLockError`
//!
//! Destroys a lock, possibly releasing it in the process.
//!

#[derive(Deserialize)]
pub struct create_lock {
	pub pid: SimplePid,
	#[serde(default)]
	pub locked: bool
}
#[derive(Clone)]
pub enum CreateLockError {
	CreateLock(String),
	LockError
}
impl<'a> RpcError<'a> for CreateLockError {
	fn code(&self) -> isize {
		match self {
			CreateLockError::CreateLock(_) => -3201,
			CreateLockError::LockError => -3202
		}
	}
	fn message(&self) -> std::borrow::Cow<'static, str> {
		match self {
			CreateLockError::CreateLock(_) => "failed to create lock".into(),
			CreateLockError::LockError => "coult not lock".into()
		}
	}

	type Data = String;
	fn data(&self) -> Option<String> {
		match self {
			CreateLockError::CreateLock(s) => Some(s.clone()),
			CreateLockError::LockError => None
		}
	}
}
impl Procedure<'static> for create_lock {
	const NAME: &'static str = "create_lock";
	type Result = crate::rpc::Null;
	type Error = CreateLockError;
}

use serde::{Serialize, Deserialize};

use procmem_access::platform::simple::SimplePid;

use crate::rpc::RpcError;

use super::Procedure;

#[derive(Serialize, Deserialize)]
pub struct CreateLockParams {
	pub pid: SimplePid,
	#[serde(default)]
	pub locked: bool
}
pub type CreateLockResult = crate::rpc::Null;


#[derive(Serialize, Deserialize)]
pub struct LockParams {
	pub pid: SimplePid
}
pub type LockResult = bool;

#[derive(Serialize, Deserialize)]
pub struct LockExclusiveParams {
	pub pid: SimplePid
}
pub type LockExclusiveResult = crate::rpc::Null;

#[derive(Serialize, Deserialize)]
pub struct UnlockParams {
	pub pid: SimplePid
}
pub type UnlockResult = bool;

#[derive(Serialize, Deserialize)]
pub struct DropParams {
	pub pid: SimplePid
}
pub type DropResult = crate::rpc::Null;