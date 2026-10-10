#!/bin/bash
# teq.lock's conformance (docs/TARGETS.md, "The export and the project verbs", the format): the YAML 1.2
# core-schema reader pinned in tests/lock (the `yaml` package, installed there by `npm ci` when
# missing, and in integrations/vite by `npm install`, where vite-plugin-teq's reader takes it),
# vite-plugin-teq's and check-export.py's readers read the corpus to corpus.json's tree and the
# example's lock to one tree, check-export.py refuses every document of refused.txt on its line and
# vite-plugin-teq's reader the ones YAML refuses or reads as no mapping, reading the others as the
# core schema does, check-export.py writes both to their bytes, and PyYAML, where installed, reads
# them alike; the Zed extension's copy of teq's reader is teq's. teq's own reader and writer are held to the same
# files by `cargo test` (task::export's the_conformance_corpus), sbt-teq's writer by its suite.
cd "$(dirname "$0")/lock" || exit 1
if [ ! -d node_modules/yaml ]; then
  timeout 120 npm ci --no-audit --no-fund > npm-ci.log 2>&1 || { echo "FAIL lock: npm ci (see tests/lock/npm-ci.log)"; exit 1; }
  rm -f npm-ci.log
fi
if [ ! -d ../../integrations/vite/node_modules/yaml ]; then
  timeout 120 npm --prefix ../../integrations/vite install --legacy-peer-deps --prefer-offline --no-audit --no-fund > npm-vite.log 2>&1 || { echo "FAIL lock: npm install in integrations/vite (see tests/lock/npm-vite.log)"; exit 1; }
  rm -f npm-vite.log
fi
status=0
timeout 60 node conformance.mjs || status=1
timeout 60 python3 conformance.py || status=1
if cmp -s ../../src/task/lock.rs ../../integrations/zed/src/lock.rs; then
  echo "lock: integrations/zed/src/lock.rs is src/task/lock.rs"
else
  echo "FAIL lock: integrations/zed/src/lock.rs differs from src/task/lock.rs: copy it again"
  status=1
fi
exit $status
