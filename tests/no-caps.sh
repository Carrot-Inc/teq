#!/bin/bash
# Numeric cutoffs added since the base anywhere under src: a loop bounded by a literal, a depth, fuel, level or step
# comparison, a length comparison or a `take` against a literal of three or more. Dotty walks a type or a tree to its
# end; a limit it does have is a named constant (`Config.LogPendingSubTypesThreshold`, `-Xmax-inlines`), the JDK,
# Scala.js, the formats teq reads and the tools its own tooling is held to (rust-analyzer, scalafix, scalameta, the
# LSP, the OS) have theirs, so a line adding a cutoff names its reference on the line (the word `dotty`, `scalac`,
# `jdk`, `scala.js`, `rfc`, `rust-analyzer`, `scalafix`, `scalameta`, `lsp`, `libc`, `posix` or `windows` on it),
# else the line is listed and the check fails. The base is master where the
# worktree holds it as an ancestor (the sharded gate's rebased worktree), else the previous commit (a landing's
# squash), the comparison from the merge base; `tests/no-caps.sh <base> <head>` compares two revisions the same way.
cd "$(dirname "$0")/.."
if [ $# -ge 1 ]; then base=$1; head=${2:-HEAD}
elif git merge-base --is-ancestor master HEAD 2>/dev/null; then base=master; head=HEAD
else base=HEAD~1; head=HEAD; git rev-parse -q --verify "$base^{commit}" > /dev/null || git fetch -q --deepen=1 2>/dev/null; fi
git rev-parse -q --verify "$base^{commit}" > /dev/null || { echo "no-caps: no base to compare ($base is not a commit here)"; exit 2; }
python3 - "$base" "$head" <<'PY'
import re, subprocess, sys
base, head = sys.argv[1], sys.argv[2]
paths = ['src']
diff = subprocess.run(['git', 'diff', '-U0', f'{base}...{head}', '--'] + paths, capture_output=True, text=True).stdout
cutoff = re.compile(r'\.\.=?\s*(\d+)\b'
                    r'|\b(?:depth|level|fuel|budget|steps?|rounds?|iterations?|limit|tries|attempts|links?|hops?|nesting|passes)\w*\s*(?:<|>|<=|>=|==|!=)\s*(\d+)\b'
                    r'|\.len\(\)\s*(?:<|>|<=|>=)\s*(\d+)\b'
                    r'|\btake\((\d+)\)'
                    r'|\bconst\s+[A-Z_]*(?:MAX|LIMIT|DEPTH|FUEL|BUDGET|CAP|BOUND)[A-Z_]*\s*:\s*(?:usize|u32|u64|i32)\s*=\s*(\d+)\b')
hits = []
file = None; n = 0
for line in diff.split('\n'):
    if line.startswith('+++ '):
        file = line[6:] if line.startswith('+++ b/') else line[4:]; continue
    m = re.match(r'@@ -\S+ \+(\d+)', line)
    if m: n = int(m.group(1)); continue
    if line.startswith('+') and not line.startswith('+++'):
        text = line[1:]
        for mm in cutoff.finditer(text):
            value = max(int(g) for g in mm.groups() if g)
            if value >= 3 and not re.search(r'dotty|scalac|jdk|scala\.?js|rfc|rust-analyzer|scalafix|scalameta|\blsp\b|libc|posix|windows', text, re.I) and not text.lstrip().startswith('//'):
                hits.append(f'{file}:{n}: {text.strip()}')
                break
        n += 1
for h in hits: print(h)
print(f'no-caps: {len(hits)} added cutoff line(s) without a reference named ({base}..{head})')
sys.exit(1 if hits else 0)
PY
