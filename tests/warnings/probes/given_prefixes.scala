// An implicit argument's prefixes: a wildcard that brings the object holding the given is used.
package lib {
  object Codecs {
    given Int = 1
  }
  object Other
}
package app {
  import lib.*
  import lib.Codecs.given
  class U { def f = summon[Int] }
}
