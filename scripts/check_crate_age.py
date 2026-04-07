#!/usr/bin/env python3
import json
import re
import sys
import urllib.request
from datetime import datetime, timezone, timedelta
from pathlib import Path

MIN_AGE_DAYS = int(sys.argv[1]) if len(sys.argv) > 1 else 7
LOCKFILE = Path("Cargo.lock")
IGNORE = {
    # std workspace/local/path deps are ignored elsewhere; keep room for exceptions here
}

if not LOCKFILE.exists():
    print("Cargo.lock not found", file=sys.stderr)
    sys.exit(1)

text = LOCKFILE.read_text()
packages = []
current = {}
for line in text.splitlines():
    line = line.strip()
    if line == '[[package]]':
        if current:
            packages.append(current)
        current = {}
        continue
    m = re.match(r'^(name|version|source) = "(.*)"$', line)
    if m:
        current[m.group(1)] = m.group(2)
if current:
    packages.append(current)

crates = []
for pkg in packages:
    source = pkg.get('source', '')
    if not source.startswith('registry+'):
        continue
    name = pkg.get('name')
    version = pkg.get('version')
    if not name or not version:
        continue
    if (name, version) in IGNORE:
        continue
    crates.append((name, version))

cutoff = datetime.now(timezone.utc) - timedelta(days=MIN_AGE_DAYS)
violations = []
checked = 0
cache = {}

for name, version in sorted(set(crates)):
    url = f"https://crates.io/api/v1/crates/{name}/{version}"
    try:
        if url in cache:
            data = cache[url]
        else:
            with urllib.request.urlopen(url, timeout=20) as resp:
                data = json.loads(resp.read().decode())
                cache[url] = data
    except Exception as e:
        print(f"WARN: failed to query crates.io for {name} {version}: {e}", file=sys.stderr)
        continue

    checked += 1
    published_at = data.get('version', {}).get('created_at')
    if not published_at:
        print(f"WARN: no created_at for {name} {version}", file=sys.stderr)
        continue

    published = datetime.fromisoformat(published_at.replace('Z', '+00:00'))
    if published > cutoff:
        age = datetime.now(timezone.utc) - published
        violations.append((name, version, published.isoformat(), age.days))

print(f"Checked {checked} registry crates against a {MIN_AGE_DAYS}-day minimum age policy.")

if violations:
    print("\nDependency age policy violations:")
    for name, version, published, age_days in violations:
        print(f"- {name} {version} published {published} ({age_days} days old)")
    sys.exit(2)

print("No age-policy violations found.")
