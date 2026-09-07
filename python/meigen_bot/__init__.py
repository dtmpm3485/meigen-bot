"""All detection, persistence, and Discord operations are implemented in Rust."""
from ._native import analyze, run, version

__all__ = ["run", "version", "analyze"]
__version__ = version()
