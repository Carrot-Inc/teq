#!/usr/bin/env python3
"""bench/ship-record.py <record> get <key> | set <key> <value>... | show: the ship's record (docs/TARGETS.md,
"Releases"), out/ship/<version>/record.json, which bench/ship-publish.sh writes before each step of a publication and
reads to resume one: a JSON object of dotted keys' values (`github.state`, `central.state`, `smoke.mirror`, ...), each
set written with the time it was set (`<key>.at`). A write goes to a file of its own, synced and renamed into place,
the directory synced, so that a run cut off leaves the record as it was or as it became, never half; a job that runs on
another machine takes the record up as an artifact of the one before (the workflow carries out/ship/ and out/central/
across). A write that fails exits nonzero, and the publication stops on it.

  get <key>               prints the value, nothing when it is not set
  set <key> <value>...    sets each key to its value (a pair at a time), with its time
  show                    prints the record
"""
import datetime
import json
import os
import sys


def load(path):
    if not os.path.isfile(path):
        return {}
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def save(path, record):
    os.makedirs(os.path.dirname(os.path.abspath(path)), exist_ok=True)
    part = path + ".part"
    with open(part, "w", encoding="utf-8") as f:
        json.dump(record, f, indent=1, sort_keys=True)
        f.write("\n")
        f.flush()
        os.fsync(f.fileno())
    os.replace(part, path)
    directory = os.open(os.path.dirname(os.path.abspath(path)), os.O_RDONLY)
    try:
        os.fsync(directory)
    finally:
        os.close(directory)


def main(argv):
    if len(argv) < 3:
        sys.exit(__doc__)
    path, command, args = argv[1], argv[2], argv[3:]
    record = load(path)
    if command == "get" and len(args) == 1:
        value = record.get(args[0])
        if value is not None:
            print(value)
    elif command == "set" and args and len(args) % 2 == 0:
        at = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
        for key, value in zip(args[::2], args[1::2]):
            record[key] = value
            record[key + ".at"] = at
        save(path, record)
    elif command == "show" and not args:
        print(json.dumps(record, indent=1, sort_keys=True))
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main(sys.argv)
