"""Copy Red to .tmp/phase_04/4_16/tracer_prefox/red_run, configure it for the pre-Fox tracer (PES 17, no deploy, default bins), and stage the export."""

import shutil
from pathlib import Path

RED = Path(r"C:\Data\4cc\4cc aet compiler\4cc-aet-compiler-red")
HERE = Path(r"C:\Data\4cc\Tools_Mine\4cc-studio\.tmp\4_16\tracer_prefox")
TARGET = HERE / "red_run"
EXPORT = HERE / "old" / "jp Tracer"

assert not TARGET.exists(), f"{TARGET} exists; remove it by hand first"
assert EXPORT.is_dir(), f"{EXPORT} missing"
shutil.copytree(
    RED,
    TARGET,
    ignore=shutil.ignore_patterns(".git", "patches_output", "extracted_teams", "temp", "*.log"),
)

settings = TARGET / "settings.ini"
text = settings.read_text(encoding="utf-8")
replacements = {
    "pes_version = 15": "pes_version = 17",
    "cpk_name = 4cc_90_test": "cpk_name = 4cc_90_tracer",
    "move_cpks = 1": "move_cpks = 0",
    r"pes_folder_path = C:\Program Files (x86)\Pro Evolution Soccer 20**": r"pes_folder_path = C:\nonexistent_pes_folder",
    "pause_allow = 1": "pause_allow = 0",
    "updates_check = 1": "updates_check = 0",
}
for old, new in replacements.items():
    assert text.count(old) == 1, old
    text = text.replace(old, new)
temporary = settings.with_suffix(".ini.new")
temporary.write_text(text, encoding="utf-8", newline="\r\n")
temporary.replace(settings)

exports = TARGET / "exports_to_add"
shutil.copytree(EXPORT, exports / EXPORT.name)
print("red ready at", TARGET)
