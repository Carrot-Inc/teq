// expect: 16:34: error: inline value must have a literal constant type
// expect: 1 error found
// absent: 14:21: error: illegal access to protected
// Pending until the phase gate (rules' R-ISRUNNABLE): a record of a group after the first group that reported an
// error is not presented. scalac reports the inline value's error (InlineVals' group) alone;
// teq also presents ProtectedAccessors' later "illegal access to protected" record (a self type's member).
package lib {
  class C(val origin: String) {
    protected def m(x: Int): String = "m" + x + origin
  }
}
package use {
  trait Side { self: lib.C =>
    def f: String = m(1)
  }
  object V { inline val x: Int = scala.util.Random.nextInt() }
}
