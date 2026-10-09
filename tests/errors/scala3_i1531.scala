// Adapted from scala3 tests/neg/i1531.scala and tests/neg/i1392.scala (Apache-2.0, see tests/scala3/README.md); replaced: nothing, the two tests are joined.
// expect: 8:7: error: class A needs to be abstract, since def f: Int in trait T is not defined
// expect: 14:20: error: Super call cannot be emitted: the selected method m is declared in class K, which is not the direct superclass of class N
// expect: 2 errors found
trait T {
  def f: Int
}
class A(f: Int) extends T

class K { def m = 1 }
class L extends K { override def m = 2 }
trait M extends K
class N extends L with M {
  override def m = super[M].m
}
