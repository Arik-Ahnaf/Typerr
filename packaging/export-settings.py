"""Stage portable defaults without distributing the builder's saved profile."""
import json
from pathlib import Path
import sys

settings = json.loads(Path(sys.argv[1]).read_text())
settings.pop("name", None)
settings.pop("course", None)
Path(sys.argv[2]).write_text(json.dumps(settings, indent=2) + "\n")
