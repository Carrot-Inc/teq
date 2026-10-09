// expect: 13:12: error: illegal cyclic type reference: upper bound T.this.b.M & U of type M refers back to the type itself
// expect: 20:21: error: type mismatch: found (m1 : x.M), required x.b.M
// expect: 21:23: error: type mismatch: found (m2 : x.b.M), required x.b.b.M
// expect: 26:12: error: illegal cyclic type reference: upper bound T.this.b.M & U
// expect: 33:21: error: type mismatch: found (m1 : x.M), required x.b.M
// expect: 6 errors found
// A bound naming its own member through a path is cyclic inside an intersection or a
// refinement too, as under scalac; the lookup of `a.x.b.b...` used not to end.
trait U
object infpaths {
  object a {
    trait T { t =>
      type M <: t.b.M & U
      type T <: a.T
      val b: t.T
    }
    val x: a.T = ???
  }
  val m1: a.x.M = ???
  val m2: a.x.b.M = m1
  val m3: a.x.b.b.M = m2
}
object refined {
  object a {
    trait T { t =>
      type M <: (t.b.M & U) { type X }
      type T <: a.T
      val b: t.T
    }
    val x: a.T = ???
  }
  val m1: a.x.M = ???
  val m2: a.x.b.M = m1
  val m3: a.x.b.b.M = m2
}
