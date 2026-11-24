#!/usr/bin/env python3
"""refused_reads.py <records> | --self-test

The judge of tests/workers.sh's refused reads (`refused_reads_as_one`): a part build at two workers with its k-th read
of a program's signature under the loader's lock refused (`TEQ_REFUSE_READ=<k>`, src/shake.rs) is given to one worker
and is one worker's build. How many such reads a build makes before it is given to one worker is its schedule's: the
first wait under the lock hands it over (read 774 where the worker typing a source built again has not finished it, read
2,728 where it has, on partial_rebuild_macro_in_initialiser). So the property is judged at every k a build reaches, and a k
no build reaches is said, not failed.

<records>: a line a build, tab-separated: `<scenario> <k> <build> <refused> <counted> <typed again> <one worker's>`,
refused, typed again and one worker's `yes` or `no`, counted the build's count of such reads or `none` (a build that
said none). A scenario's builds of a k stop at the first that refuses read k. Its verdict:
- red: a build that is not one worker's build (its exit, text or directory), whether it refused read k or was handed
  over earlier; a build that refused read k and was not typed again by one worker; a build that counted k or more
  reads and refused none of them; five builds of a k said no count (a teq that
  says no count of its reads, an older one); no k of the scenario reached in any build;
- else ok, each k no build reached said with the counts its builds gave.
Prints a line a k, how many of its builds reached it (refused it, or counted k reads or more) and the counts of those
that did not, so that a run's text shows at which reads the property was judged on its machine; and one a scenario
(`ok <scenario>: judged at <r> of <n> reads named` or `red <scenario>: <why>`). Exits 1 on any red.
"""
import collections
import sys

BUILDS = 5


def judge(records):
    by = collections.OrderedDict()
    for r in records:
        by.setdefault(r[0], collections.OrderedDict()).setdefault(int(r[1]), []).append(r)
    out, red = [], False
    for scenario, ks in by.items():
        why, reached = [], 0
        for k, builds in ks.items():
            refused = [b for b in builds if b[3] == "yes"]
            counts = [b[4] for b in builds]
            hit = [b for b in builds if b[3] == "yes" or (b[4] != "none" and int(b[4]) >= k)]
            short = [b[4] for b in builds if b not in hit]
            out.append(f"read {k} of {scenario}: reached by {len(hit)} of {len(builds)} builds" + (f"; the others counted {', '.join(short)}" if short else ""))
            other = [b for b in builds if b[6] != "yes"]
            if other:
                why.append(f"read {k}: build {other[0][2]} is not one worker's" + (" (it refused the read)" if other[0][3] == "yes" else f" (it counted {other[0][4]} reads)"))
                continue
            if refused:
                b = refused[0]
                reached += 1
                if b[5] != "yes":
                    why.append(f"read {k}: refused and not typed again by one worker")
                else:
                    out.append(f"ok {scenario} read {k}" + (f" (after builds that counted {', '.join(counts[:-1])})" if len(builds) > 1 else ""))
                continue
            over = [c for c in counts if c != "none" and int(c) >= k]
            if over:
                why.append(f"read {k}: a build counted {over[0]} reads and refused none of them")
            elif len(builds) >= BUILDS and all(c == "none" for c in counts):
                why.append(f"read {k}: {len(builds)} builds said no count of their reads")
            elif len(builds) >= BUILDS:
                out.append(f"not reached {scenario} read {k}: the builds counted {', '.join(counts)}")
            else:
                why.append(f"read {k}: {len(builds)} builds, none refused it, fewer than {BUILDS}")
        if not why and reached == 0:
            why.append("no read named was reached in any build: the scenario judged nothing")
        if why:
            red = True
            out.append(f"red {scenario}: " + "; ".join(why))
        else:
            out.append(f"ok {scenario}: judged at {reached} of {len(ks)} reads named, each refused and one worker's")
    return out, red


def self_test():
    def rec(s, k, n, refused, counted, again="yes", one="yes"):
        return (s, str(k), str(n), refused, str(counted), again, one)
    reach = lambda s, k: [rec(s, k, 1, "yes", 2688)]
    miss = lambda s, k, c: [rec(s, k, n, "no", c) for n in range(1, BUILDS + 1)]
    checks = [
        ("every k reached and one worker's", reach("a", 1) + reach("a", 2), False),
        ("a k past every build's count, said", reach("a", 1) + miss("a", 987, 774), False),
        ("a build counted k or more and refused none", reach("a", 1) + [rec("a", 987, 1, "no", 2728)], True),
        ("refused, not typed again", [rec("a", 1, 1, "yes", 8, again="no")], True),
        ("refused, another text or directory", [rec("a", 1, 1, "yes", 8, one="no")], True),
        ("handed over before read k, and not one worker's", reach("a", 1) + [rec("a", 987, 1, "no", 774, one="no")], True),
        ("nothing reached in any build", miss("a", 1, 0) + miss("a", 2, 0), True),
        ("five builds with no count", reach("a", 1) + miss("a", 987, "none"), True),
        ("a teq that never refuses: counts, no refusal at k=1", [rec("a", 1, 1, "no", 774)], True),
        ("reached after builds that were handed over earlier", [rec("a", 987, 1, "no", 774), rec("a", 987, 2, "yes", 2688)], False),
        ("one scenario's red is not another's", reach("a", 1) + [rec("b", 1, 1, "no", 774)], True),
    ]
    bad = []
    for what, records, want_red in checks:
        out, red = judge(records)
        if red != want_red:
            bad.append(f"{what}: {'red' if red else 'ok'}, not {'red' if want_red else 'ok'}: {out}")
    out, red = judge(reach("a", 1) + miss("a", 987, 774))
    if not any(l.startswith("not reached a read 987: the builds counted 774") for l in out):
        bad.append(f"the unreached k is not said with its counts: {out}")
    if "read 987 of a: reached by 0 of 5 builds; the others counted 774, 774, 774, 774, 774" not in out or "ok a: judged at 1 of 2 reads named, each refused and one worker's" not in out:
        bad.append(f"how many builds reached each read, and at how many the scenario judged, are not said: {out}")
    out, red = judge([rec("a", 987, 1, "no", 774), rec("a", 987, 2, "yes", 2688)])
    if "read 987 of a: reached by 1 of 2 builds; the others counted 774" not in out:
        bad.append(f"a read reached after a build handed over earlier is not said so: {out}")
    for b in bad:
        print("refused_reads self-test FAIL:", b)
    print(f"refused_reads self-test: {len(checks) + 3} planted records, {len(bad)} not as they must be")
    return 1 if bad else 0


if __name__ == "__main__":
    if sys.argv[1:] == ["--self-test"]:
        sys.exit(self_test())
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    records = [l.rstrip("\n").split("\t") for l in open(sys.argv[1]) if l.strip()]
    out, red = judge(records)
    print("\n".join(out))
    sys.exit(1 if red else 0)
