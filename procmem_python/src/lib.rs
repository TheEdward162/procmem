use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

use procmem_access;

// #[pyfunction]
// fn sum_as_string(a: usize, b: usize) -> PyResult<String> {
//     Ok((a + b).to_string())
// }

/// Procmem python bindings.
#[pymodule]
fn procmem_python(_py: Python, m: &PyModule) -> PyResult<()> {
    // m.add_function(
	// 	wrap_pyfunction!(sum_as_string, m)?
	// )?;

    Ok(())
}
