# release-fixture.py <releases dir> <version> <word> [<classifier>...]: a release of the checks' own (check.sh's
# release section, check-export.sh's releases) under <releases dir>/v<version>/, as bench/github-release.sh lays one
# out and bench/release-mirror.py serves it: each classifier's asset teq-<version>-<classifier> (.exe for Windows
# alone), a shell script printing `teq <version> <word>`, then SHA256SUMS and the binary manifest
# teq-<version>-binaries.txt from the same bytes; every classifier a release carries when none is named. Prints a
# line `<classifier> <asset> <sha1> <size>` for each.
import hashlib
import os
import sys

CLASSIFIERS = ["osx-aarch_64", "osx-x86_64", "linux-x86_64", "linux-aarch_64", "windows-x86_64"]

root, version, word, *classifiers = sys.argv[1:]
directory = os.path.join(root, "v" + version)
os.makedirs(directory, exist_ok=True)
manifest, sums = [f"teq {version} {'0' * 40}"], []
for classifier in classifiers or CLASSIFIERS:
    asset = f"teq-{version}-{classifier}" + (".exe" if classifier.startswith("windows") else "")
    body = f"#!/bin/sh\necho 'teq {version} {word}'\n".encode()
    with open(os.path.join(directory, asset), "wb") as f:
        f.write(body)
    sha256, sha1 = hashlib.sha256(body).hexdigest(), hashlib.sha1(body).hexdigest()
    manifest.append(f"{classifier} {asset} {sha256} {sha1} {len(body)}")
    sums.append(f"{sha256}  {asset}")
    print(f"{classifier} {asset} {sha1} {len(body)}")
with open(os.path.join(directory, f"teq-{version}-binaries.txt"), "w") as f:
    f.write("\n".join(manifest) + "\n")
with open(os.path.join(directory, "SHA256SUMS"), "w") as f:
    f.write("\n".join(sums) + "\n")
