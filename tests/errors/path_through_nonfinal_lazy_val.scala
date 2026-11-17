class Ops(val seed: Int):
  class C(val v: Int) { def ==(o: C): Boolean = v == o.v }
  lazy val lz: Ops = new Ops(seed + 1)
def uses(): Unit =
  lazy val lo = new Ops(2)
  println(Array(new lo.lz.C(1)).length)
  println(new lo.lz.C(1) == new lo.lz.C(1))
// A path through a lazy val that is not final is no legal path: scalac refuses each use of it, a class named
// through it included.
// expect: 6:24: error: (lo.lz : Ops) is not a legal path
// expect: since it refers to nonfinal lazy value lz
// expect: 7:18: error: (lo.lz : Ops) is not a legal path
// expect: 7:36: error: (lo.lz : Ops) is not a legal path
// expect: 3 errors found
