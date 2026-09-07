use pyo3::{
    exceptions::{PyRuntimeError, PyValueError},
    prelude::*,
    types::PyDict,
};
use std::{path::PathBuf, sync::mpsc, time::Duration};

#[pyfunction]
fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
#[pyfunction]
fn analyze<'py>(py: Python<'py>, text: &str) -> PyResult<Bound<'py, PyDict>> {
    let a = crate::detector::analyze_message(text);
    let d = PyDict::new(py);
    d.set_item("is_meigen", a.is_meigen)?;
    d.set_item("score", a.score)?;
    d.set_item("deep_score", a.deep_score)?;
    d.set_item("funny_score", a.funny_score)?;
    d.set_item("persuasion_score", a.persuasion_score)?;
    d.set_item("degenerate_score", a.degenerate_score)?;
    d.set_item("genres", a.genres)?;
    d.set_item("reasons", a.reasons)?;
    Ok(d)
}
/// Starts the blocking bot. Use asyncio.to_thread(run, token) from asyncio.
#[pyfunction]
#[pyo3(signature=(token, database="meigen.db"))]
fn run(py: Python<'_>, token: String, database: &str) -> PyResult<()> {
    if token.trim().is_empty() || token.chars().any(char::is_whitespace) {
        return Err(PyValueError::new_err(
            "token must be a nonempty Discord Bot token without whitespace",
        ));
    }
    if database.is_empty() {
        return Err(PyValueError::new_err("database path must not be empty"));
    }
    let path = PathBuf::from(database);
    let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
    let (done_tx, done_rx) = mpsc::channel();
    let handle = std::thread::Builder::new()
        .name("meigen-runtime".into())
        .spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let runtime = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .worker_threads(2)
                    .build()
                    .map_err(|e| e.to_string())?;
                let result = runtime
                    .block_on(crate::bot::start(token, path, stop_rx))
                    .map_err(|e| e.to_string());
                runtime.shutdown_timeout(Duration::from_secs(4));
                result
            }))
            .unwrap_or_else(|_| Err("Rust bot worker panicked".into()));
            let _ = done_tx.send(result);
        })
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
    let done_rx = std::sync::Mutex::new(done_rx);
    let mut signal_error = None;
    let result = loop {
        match py.allow_threads(|| {
            done_rx
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .recv_timeout(Duration::from_millis(100))
        }) {
            Ok(result) => break result,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                break Err("Rust worker disconnected".into())
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        if signal_error.is_none() {
            if let Err(e) = py.check_signals() {
                let _ = stop_tx.send(true);
                signal_error = Some(e);
            }
        }
    };
    let _ = py.allow_threads(|| handle.join());
    if let Some(e) = signal_error {
        return Err(e);
    }
    result.map_err(PyRuntimeError::new_err)
}
#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(run, m)?)?;
    m.add_function(wrap_pyfunction!(version, m)?)?;
    m.add_function(wrap_pyfunction!(analyze, m)?)?;
    Ok(())
}
