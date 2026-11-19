// expect: 11:34: error: inline value must have a literal constant type
// expect: 1 error found
// absent: 12:20: error: illegal access to protected
// Pending until the phase gate (rules' R-ISRUNNABLE): a record of a group after the first group that reported an
// error is not presented. scalac reports the inline value's error (InlineVals' group) alone;
// teq also presents ProtectedAccessors' later "illegal access to protected" record (a constructor's default getter).
package store {
  class Dflt protected (val a: Int = 1)
}
package shop {
  object V { inline val x: Int = scala.util.Random.nextInt() }
  class D1 extends store.Dflt
}
