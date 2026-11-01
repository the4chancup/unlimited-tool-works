"""The wheel's smoke test (core plan, "Phase 2", `python_bindings`): run by `just bindings`
through `scripts/gates/bindings_check.py` with the built wheel on `sys.path`, under any Python 3.11+
(the developer's or Blender's). Not a pytest suite: Blender's Python has no pytest.

The oracle is each format crate's own invariant (`fmdl/src/format/file.rs`, `skl.rs`,
`pes_model/src/format/model/tests.rs`, `mtl.rs`): byte identity where the crate proves it,
write-idempotence elsewhere, and a `FormatError` on junk. Fixture counts are literals so a
missing directory cannot pass as "nothing to check".
"""

import logging
import sys
from pathlib import Path

import pes_models_native as native

LIBS = Path(__file__).resolve().parents[2]
FMDL_FIXTURES = LIBS / "fmdl" / "tests" / "fixtures"
MODEL_FIXTURES = LIBS / "pes_model" / "tests" / "fixtures"

# The `.mtl` files `mtl.rs`'s `regular_fixtures_write_byte_for_byte` proves
# byte-identical; the rest round-trip semantically only.
MTL_BYTE_IDENTICAL = frozenset({
    "cardhead_materials.mtl",
    "konami_headHi.mtl",
    "konami_hair.mtl",
    "konami_shadow.mtl",
})

# (class, fixture files, expected count, byte-identity predicate on the file name)
CASES = [
    (native.fmdl.Fmdl, sorted(FMDL_FIXTURES.glob("*.fmdl")), 5,
     lambda name: name.startswith("konami_")),
    (native.fmdl.Skl, sorted(FMDL_FIXTURES.glob("*.skl")), 4, lambda name: True),
    (native.pes_model.Model, sorted(MODEL_FIXTURES.glob("*.model")), 18, lambda name: False),
    (native.pes_model.MaterialSet, sorted(MODEL_FIXTURES.glob("*.mtl")), 10,
     lambda name: name in MTL_BYTE_IDENTICAL),
]


def check_round_trips() -> int:
    mtl_names = {path.name for path in MODEL_FIXTURES.glob("*.mtl")}
    assert MTL_BYTE_IDENTICAL <= mtl_names, (
        f"MTL_BYTE_IDENTICAL names no fixture supplies: {MTL_BYTE_IDENTICAL - mtl_names}")
    checked = 0
    for cls, files, expected_count, identical in CASES:
        assert len(files) == expected_count, f"{cls.__name__}: {len(files)} fixtures, expected {expected_count}"
        for path in files:
            data = path.read_bytes()
            first = cls.read(data).write()
            assert isinstance(first, bytes), f"{path.name}: write() returned {type(first).__name__}"
            assert first, f"{path.name}: write() returned no bytes"
            assert cls.read(first).write() == first, f"{path.name}: write is not idempotent"
            if identical(path.name):
                assert first == data, f"{path.name}: rewrite differs from the file"
            checked += 1
    return checked


def check_errors() -> None:
    # FormatError's message is the Rust `Display` text: `FmdlError::Truncated`,
    # `FmdlError::BadSklMagic` (`b"not "` as a little-endian u32),
    # `ModelError::Truncated`, `MtlError::Xml` (roxmltree's position).
    cases = [
        (native.fmdl.Fmdl, "fmdl is truncated"),
        (native.fmdl.Skl, "invalid skl magic: 544501614"),
        (native.pes_model.Model, "model is truncated"),
        (native.pes_model.MaterialSet, "invalid xml: unknown token at 1:1"),
    ]
    for cls, expected in cases:
        try:
            cls.read(b"not a model file")
        except native.FormatError as error:
            assert str(error) == expected, f"{cls.__name__}: {error!r}, expected {expected!r}"
        else:
            raise AssertionError(f"{cls.__name__}.read accepted junk")
    assert issubclass(native.FormatError, Exception)


def check_logging() -> None:
    # pyo3-log is installed at import: the module's `log` output lands in Python's `logging`,
    # so a handler attached to the root logger sees it. The libs log nothing on a clean read,
    # which is what this asserts (a chatty codec would flood Blender's console).
    records: list[logging.LogRecord] = []

    class Capture(logging.Handler):
        def emit(self, record: logging.LogRecord) -> None:
            records.append(record)

    root = logging.getLogger()
    handler = Capture(level=logging.DEBUG)
    root.addHandler(handler)
    previous = root.level
    root.setLevel(logging.DEBUG)
    try:
        native.fmdl.Fmdl.read((FMDL_FIXTURES / "konami_mouth.fmdl").read_bytes())
    finally:
        root.removeHandler(handler)
        root.setLevel(previous)
    noisy = [r for r in records if r.levelno >= logging.WARNING]
    assert not noisy, f"warnings on a clean read: {[r.getMessage() for r in noisy]}"


def main() -> int:
    checked = check_round_trips()
    check_errors()
    check_logging()
    print(f"smoke: {checked} fixtures round-tripped under Python {sys.version.split()[0]} "
          f"({native.__file__})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
