// expect: 9:12: error: illegal cyclic type reference: upper bound T.this.b.M of type M refers back to the type itself
// expect: 16:21: error: type mismatch: found (infpaths.m1 : infpaths.a.x.M), required infpaths.a.x.b.M
// expect: 17:23: error: type mismatch: found (infpaths.m2 : infpaths.a.x.b.M), required infpaths.a.x.b.b.M
// expect: 3 errors found
// A bound naming its own member through a path of the member's class is cyclic, as under scalac.
object infpaths {
  object a {
    trait T { t =>
      type M <: t.b.M
      type T <: a.T
      val b: t.T
    }
    val x: a.T = ???
  }
  val m1: a.x.M = ???
  val m2: a.x.b.M = m1
  val m3: a.x.b.b.M = m2
}
