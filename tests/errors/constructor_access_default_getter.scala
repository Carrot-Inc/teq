// expect: 32:20: error: illegal access to protected method $lessinit$greater$default$1 in object Dflt from class D1
// expect: 33:21: error: illegal access to protected method $lessinit$greater$default$1 in object Dflt from object D2
// expect: 35:17: error: illegal access to protected method $lessinit$greater$default$1 in object Dflt from class Sub
// expect: 36:20: error: illegal access to protected method $lessinit$greater$default$1 in object Multi from class M2
// expect: 37:20: error: illegal access to protected method $lessinit$greater$default$2 in object Multi from class M3
// expect: 39:14: error: illegal access to protected method $lessinit$greater$default$1 in object Dflt from object Use
// expect: 40:14: error: illegal access to protected method $lessinit$greater$default$1 in object Dflt from object Use
// expect: 41:18: error: illegal access to protected method $lessinit$greater$default$1 in object Dflt from object Use
// expect: 42:14: error: illegal access to protected method $lessinit$greater$default$1 in object DfltQ from object Use
// expect: 43:14: error: illegal access to protected method $lessinit$greater$default$1 in object Sec from object Use
// expect: 44:14: error: illegal access to protected method $lessinit$greater$default$1 in object Multi from object Use
// expect: 45:14: error: illegal access to protected method $lessinit$greater$default$1 in object Multi from object Use
// expect: 12 errors found
// A protected constructor's default getter is the companion's protected method, reached outside the constructor's
// package or qualifier by no accessor (scalac's transform/ProtectedAccessors: "illegal access to protected method
// $lessinit$greater$default$1 in object Dflt from class D1"), in a parent call and in a creation alike: the line
// of a later phase of scalac's, told while the program has no earlier error (these twelve are scalac's own over
// this file). Two defaults of one application stand at one place, where its reporter keeps the first; inside the
// constructor's package, and with the argument given, nothing is told.
package store:
  class Dflt protected (val a: Int = 1)
  class DfltQ protected[store] (val a: Int = 1)
  class Sec(val a: Int, val b: Int):
    protected def this(a: Int = 5) = this(a, a)
  class Multi protected (val a: Int = 1, val b: Int = 2)
  class Near extends Dflt
  object Here:
    val h1 = new Dflt {}
    val h2 = new Multi() {}
package shop:
  import store.*
  class D1 extends Dflt
  object D2 extends Dflt
  class Sub extends Dflt(3):
    def inSub = new Dflt {}
  class M2 extends Multi
  class M3 extends Multi(5)
  object Use:
    val d1 = new Dflt {}
    val d2 = new Dflt() {}
    val d3 = new Dflt { override def toString = "d3" }
    val d4 = new DfltQ {}
    val d5 = new Sec() {}
    val m1 = new Multi {}
    val m2 = new Multi(b = 7) {}
    val ok = new Dflt(2) {}
