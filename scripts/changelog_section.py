#!/usr/bin/env python3
"""Print the CHANGELOG.md section for a version (used as the Release body)."""
import re, sys
version = sys.argv[1]
text = open("CHANGELOG.md", encoding="utf-8").read()
m = re.search(rf"^## \[{re.escape(version)}\][^\n]*\n(.*?)(?=^## \[|^\[[^\]]+\]: )", text, re.S | re.M)
print(m.group(1).strip() if m else f"Release {version}")
