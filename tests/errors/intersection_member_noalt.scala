// expect: 8:42: error: type mismatch: found "z", required "k" | "v"
// expect: 9:47: error: type mismatch: found "z", required "v" | "k"
// expect: 2 errors found
// A call no instance's parameter takes: the member seen from the whole receiver takes the union
// of the instances' arguments (`NamedType.argForParam` of the intersection's base type), so the
// mismatch names that union, as scalac's E007 does.
trait R[K <: String] { def get(k: K): Any }
def bad(r: R["k"] & R["v"]): Any = r.get("z")
def reversed(r: R["v"] & R["k"]): Any = r.get("z")
