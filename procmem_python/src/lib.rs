use std::collections::HashSet;

use pyo3::{
	exceptions::PyValueError,
	prelude::*,
	types::{PyAny, PyList},
};

use procmem_access::{
	platform::simple::{ProcessInfo, SimpleMemoryAccess, SimpleMemoryLock, SimpleMemoryMap},
	prelude::{MemoryAccess, MemoryLock, MemoryMap, MemoryPage, MemoryPagePermissions, OffsetType},
};
use procmem_scan::prelude::{ByteComparable, StreamScanner, ValuePredicate};

fn err_to_pyerr<T: std::fmt::Display>(err: T) -> PyErr {
	PyValueError::new_err(err.to_string())
}

pub type PyOffsetType = u64;

#[allow(non_camel_case_types)]
pub enum MemValue {
	i8(i8),
	i16(i16),
	i32(i32),
	i64(i64),
	f32(f32),
	f64(f64),
	String(String),
}
impl MemValue {
	pub fn try_from_py(value: &PyAny, value_type: &str) -> PyResult<Self> {
		let me = match value_type {
			"i64" => Self::i64(value.extract::<i64>()?),
			"i32" => Self::i32(value.extract::<i32>()?),
			"i16" => Self::i16(value.extract::<i16>()?),
			"i8" => Self::i8(value.extract::<i8>()?),
			"f32" => Self::f32(value.extract::<f32>()?),
			"f64" => Self::f64(value.extract::<f64>()?),
			"str" => Self::String(value.extract::<&str>()?.to_string()),
			unknown => {
				return Err(PyValueError::new_err(format!(
					"Unknown type \"{}\"",
					unknown
				)))
			}
		};

		Ok(me)
	}
}
impl ByteComparable for MemValue {
	fn as_bytes(&self) -> &[u8] {
		match self {
			Self::i8(v) => v.as_bytes(),
			Self::i16(v) => v.as_bytes(),
			Self::i32(v) => v.as_bytes(),
			Self::i64(v) => v.as_bytes(),
			Self::f32(v) => v.as_bytes(),
			Self::f64(v) => v.as_bytes(),
			Self::String(v) => v.as_str().as_bytes(),
		}
	}

	fn align_of(&self) -> usize {
		match self {
			Self::i8(v) => v.align_of(),
			Self::i16(v) => v.align_of(),
			Self::i32(v) => v.align_of(),
			Self::i64(v) => v.align_of(),
			Self::f32(v) => v.align_of(),
			Self::f64(v) => v.align_of(),
			Self::String(v) => v.as_str().align_of(),
		}
	}
}
impl IntoPy<PyObject> for MemValue {
	fn into_py(self, py: Python<'_>) -> PyObject {
		match self {
			Self::i8(v) => v.into_py(py),
			Self::i16(v) => v.into_py(py),
			Self::i32(v) => v.into_py(py),
			Self::i64(v) => v.into_py(py),
			Self::f32(v) => v.into_py(py),
			Self::f64(v) => v.into_py(py),
			Self::String(v) => v.into_py(py),
		}
	}
}

#[pyclass(name = "ProcmemSimple")]
pub struct PyProcmemSimple {
	pid: i32,
	lock: SimpleMemoryLock,
	map: SimpleMemoryMap,
	access: SimpleMemoryAccess,
	user_locked: bool,
}
#[pymethods]
impl PyProcmemSimple {
	#[new]
	pub fn new(pid: i32) -> PyResult<Self> {
		let lock = SimpleMemoryLock::new(pid).map_err(err_to_pyerr)?;
		let map = SimpleMemoryMap::new(pid).map_err(err_to_pyerr)?;
		let access = SimpleMemoryAccess::new(pid).map_err(err_to_pyerr)?;

		Ok(Self {
			pid,
			lock,
			map,
			access,
			user_locked: false,
		})
	}

	pub fn process_info(&self) -> PyProcessInfo {
		ProcessInfo::for_pid(self.pid).unwrap().into()
	}

	pub fn pages(&self) -> Vec<PyMemoryPage> {
		self.map
			.pages()
			.into_iter()
			.cloned()
			.map(PyMemoryPage::from)
			.collect()
	}

	pub fn stop(&mut self) {
		if self.user_locked {
			return;
		}
		self.user_locked = true;

		self.lock.lock().unwrap();
	}

	pub fn start(&mut self) {
		if !self.user_locked {
			return;
		}
		self.user_locked = false;

		self.lock.unlock().unwrap();
	}

	pub fn is_stopped(&self) -> bool {
		self.user_locked
	}

	#[pyo3(signature = (pages, value, value_type = "i32", aligned = true))]
	pub fn scan_exact(
		&mut self,
		pages: &PyList,
		value: &PyAny,
		value_type: &str,
		aligned: bool,
	) -> PyResult<HashSet<PyOffsetType>> {
		self.lock.lock().map_err(err_to_pyerr)?;

		let value = MemValue::try_from_py(value, value_type)?;

		let predicate = ValuePredicate::new(value, aligned);
		let mut scanner = StreamScanner::new(predicate);

		let mut matches = HashSet::new();
		let mut chunk_buffer = Vec::new();
		for page in pages {
			let page: &PyCell<PyMemoryPage> = page.downcast()?;
			let page = page.borrow();

			chunk_buffer.resize(page.size() as usize, 0u8);

			unsafe {
				self.access
					.read(page.0.start(), chunk_buffer.as_mut())
					.map_err(err_to_pyerr)?;
			}

			matches.extend(
				scanner
					.scan_once(page.0.start(), chunk_buffer.iter().copied())
					.map(|(offset, _)| offset.get()),
			);
		}

		self.lock.unlock().map_err(err_to_pyerr)?;

		Ok(matches)
	}

	#[pyo3(signature = (offset, value_type = "i32"))]
	pub fn read(&mut self, offset: PyOffsetType, value_type: &str) -> PyResult<MemValue> {
		self.lock.lock().map_err(err_to_pyerr)?;

		let offset = OffsetType::new_unwrap(offset);

		macro_rules! read_fixed_size {
			($fixed_type: ident) => {{
				let mut buffer = [0u8; std::mem::size_of::<$fixed_type>()];
				unsafe {
					self.access
						.read(offset, &mut buffer)
						.map_err(err_to_pyerr)?
				};
				MemValue::$fixed_type(<$fixed_type>::from_ne_bytes(buffer))
			}};
		}
		let value = match value_type {
			"i64" => read_fixed_size!(i64),
			"i32" => read_fixed_size!(i32),
			"i16" => read_fixed_size!(i16),
			"i8" => read_fixed_size!(i8),
			"f32" => read_fixed_size!(f32),
			"f64" => read_fixed_size!(f64),
			"str" => todo!(),
			unknown => {
				return Err(PyValueError::new_err(format!(
					"Unknown type \"{}\"",
					unknown
				)))
			}
		};

		self.lock.unlock().map_err(err_to_pyerr)?;
		Ok(value)
	}

	#[pyo3(signature = (offset, value, value_type = "i32"))]
	pub fn write(&mut self, offset: PyOffsetType, value: &PyAny, value_type: &str) -> PyResult<()> {
		self.lock.lock().map_err(err_to_pyerr)?;

		let offset = OffsetType::new_unwrap(offset);
		let value = MemValue::try_from_py(value, value_type)?;

		unsafe {
			self.access
				.write(offset, value.as_bytes())
				.map_err(err_to_pyerr)?
		};

		self.lock.unlock().map_err(err_to_pyerr)?;
		Ok(())
	}
}

#[pyclass(name = "MemoryPage")]
pub struct PyMemoryPage(MemoryPage);
impl From<MemoryPage> for PyMemoryPage {
	fn from(value: MemoryPage) -> Self {
		Self(value)
	}
}
#[pymethods]
impl PyMemoryPage {
	pub fn __str__(&self) -> String {
		self.0.to_string()
	}

	#[getter]
	pub fn start(&self) -> u64 {
		self.0.start().get()
	}

	#[getter]
	pub fn end(&self) -> u64 {
		self.0.end().get()
	}

	#[getter]
	pub fn size(&self) -> u64 {
		self.0.size()
	}

	#[getter]
	pub fn permissions(&self) -> PyMemoryPagePermissions {
		self.0.permissions.into()
	}

	#[getter]
	pub fn offset(&self) -> u64 {
		self.0.offset
	}

	#[getter]
	pub fn page_type(&self) -> String {
		self.0.page_type.to_string()
	}
}

#[pyclass(name = "MemoryPagePermissions")]
#[derive(Clone)]
pub struct PyMemoryPagePermissions(MemoryPagePermissions);
impl From<MemoryPagePermissions> for PyMemoryPagePermissions {
	fn from(value: MemoryPagePermissions) -> Self {
		Self(value)
	}
}
#[pymethods]
impl PyMemoryPagePermissions {
	pub fn __str__(&self) -> String {
		self.0.to_string()
	}

	#[getter]
	pub fn read(&self) -> bool {
		self.0.read()
	}

	#[getter]
	pub fn write(&self) -> bool {
		self.0.write()
	}

	#[getter]
	pub fn exec(&self) -> bool {
		self.0.exec()
	}

	#[getter]
	pub fn shared(&self) -> bool {
		self.0.shared()
	}
}

#[pyclass(get_all, name = "ProcessInfo")]
pub struct PyProcessInfo {
	pub pid: i32,
	pub name: String,
}
impl From<ProcessInfo> for PyProcessInfo {
	fn from(value: ProcessInfo) -> Self {
		Self {
			pid: value.pid,
			name: value.name,
		}
	}
}
#[pymethods]
impl PyProcessInfo {
	#[staticmethod]
	pub fn list_all() -> PyResult<Vec<Self>> {
		Ok(ProcessInfo::list_all()
			.map_err(err_to_pyerr)?
			.into_iter()
			.map(PyProcessInfo::from)
			.collect())
	}

	pub fn __str__(&self) -> String {
		format!("{} ({})", self.pid, self.name)
	}
}

/// Procmem python bindings
#[pymodule]
fn procmem(_py: Python, m: &PyModule) -> PyResult<()> {
	m.add_class::<PyProcmemSimple>()?;
	m.add_class::<PyMemoryPage>()?;
	m.add_class::<PyMemoryPagePermissions>()?;
	m.add_class::<PyProcessInfo>()?;

	Ok(())
}
