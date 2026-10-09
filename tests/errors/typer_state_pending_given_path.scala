// expect: 11:19: error: type mismatch: found C, required (a : C)
// expect: 1 error found
// A plain inline given still pending is no path: each of its expansions makes a value of its
// own, so the second `get` is not `a.type`, as scalac reports.
class C
inline given c: C = new C
def get(using x: C): x.type = x

@main def main(): Unit =
  val a = get
  val b: a.type = get
  println(a eq b)
