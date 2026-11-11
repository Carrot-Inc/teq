// A `new` of a type lambda over an opaque type has no constructor: typeOfNew collapses an eta-expansion of a class
// only (`EtaExpansion` asks `fn.typeSymbol.isClass`, TypeApplications.scala:98-103), and an opaque type is none. The
// lambda over the class is a `new` of the class. As scalac 3.8.4 reports it; scalac's caret begins at the
// parenthesized type's `(` (9:15), teq's at the lambda (9:16), a position of the parenthesized type tree's own.
// expect: error: [A] =>> O.F[A] does not have a constructor
// expect: 1 error found
class Box[A]
object O:
  opaque type F[A] = Box[A]
def bad = new ([A] =>> O.F[A])()
def good = new ([A] =>> Box[A])()
