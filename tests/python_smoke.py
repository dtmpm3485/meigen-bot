"""Runs against the installed native wheel; no Discord credentials or connection."""
import asyncio
import re
import tempfile
from meigen_bot import run, version, analyze

assert re.fullmatch(r"\d+\.\d+\.\d+", version())
a = analyze("学校には遅刻するけどログボには遅刻しない")
assert a["is_meigen"] and a["score"] >= 75
assert not analyze("今日ゲームする？")["is_meigen"]
for token in ("", "bad token"):
    try:
        run(token)
    except ValueError:
        continue
    raise AssertionError("invalid token accepted")

async def check_runtime():
    try:
        await asyncio.to_thread(run, "")
    except ValueError:
        return
    raise AssertionError("invalid token accepted in asyncio")

asyncio.run(check_runtime())
# A directory used as a database must fail inside the Rust worker, before Discord.
with tempfile.TemporaryDirectory() as directory:
    try:
        run("not-a-real-token", database=directory)
    except RuntimeError as exc:
        assert "SQLite" in str(exc)
    else:
        raise AssertionError("unwritable DB accepted")

async def check_worker_inside_asyncio():
    with tempfile.TemporaryDirectory() as directory:
        try:
            await asyncio.to_thread(run, "not-a-real-token", database=directory)
        except RuntimeError as exc:
            assert "SQLite" in str(exc)
            return
        raise AssertionError("Rust worker did not return an exception")

asyncio.run(check_worker_inside_asyncio())
# Lone Python surrogates cannot become Rust UTF-8 strings and must raise, not crash.
try:
    analyze("\ud800")
except UnicodeEncodeError:
    surrogate_rejected = True
else:
    raise AssertionError("surrogate unexpectedly accepted")
print("native wheel smoke tests succeeded", version())
