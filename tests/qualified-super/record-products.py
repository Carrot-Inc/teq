#!/usr/bin/env python3
"""Record Scala 3.8.4's JVM and Scala.js clients, regenerated class members and qualifier pickles."""
from pathlib import Path
import tempfile

from products import (LEGS, OUT, REFERENCE, compile_jvm_client, from_tasty, libraries, pickle_members,
                      require, run_jvm_client, scala_command, scalac_library, split_library)
from run import field


def main():
    lines = ["# library\tleg\tscalac 3.8.4 observable"]
    OUT.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=OUT) as tmp:
        for name, source in libraries():
            row = split_library(Path(tmp), name, source)
            classes = scalac_library(row)
            client = row / "client-scalac"
            require(row, "scalac client", compile_jvm_client(row, classes, client))
            jvm = require(row, LEGS[0], run_jvm_client(row, client, classes))
            js = require(row, "Scala.js", scala_command(row, ["scala-cli", "run", "--js", "-S", "3.8.4",
                         "--server=false", "-color:never", "Library.scala", "Client.scala"]))
            if not jvm or len(jvm.splitlines()) != 1 or "\t" in jvm or jvm != js:
                raise ValueError(f"{name}: JVM {jvm!r}, Scala.js {js!r}")
            regenerated, members = from_tasty(row, classes, client)
            listing, entry = pickle_members(row, classes)
            values = (jvm, require(row, LEGS[1], regenerated), js, require(row, LEGS[3], listing),
                      require(row, LEGS[4], members), require(row, LEGS[5], entry))
            lines.extend(f"{name}\t{leg}\t{field(value)}" for leg, value in zip(LEGS, values))
    REFERENCE.write_text("\n".join(lines) + "\n")
    print(f"record-products.py: {len(lines) - 1} rows; expected-products.tsv written")


if __name__ == "__main__":
    main()
