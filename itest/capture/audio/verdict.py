import importlib.util
import json
from pathlib import Path
import sys


def verdict(value, output: Path) -> dict[str, str]:
    spec = importlib.util.spec_from_file_location("audio_capture", Path(__file__).with_name("audio_capture.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    directory = output / "audio"
    directory.mkdir(exist_ok=True)
    result = module.evaluate(value, output)
    (directory / "audio-verdict.json").write_text(json.dumps(result, indent=2, allow_nan=False) + "\n", encoding="utf-8")
    for error in result["errors"]:
        print(f"audio: {error}", file=sys.stderr)
    return {"audio": result["status"]}
