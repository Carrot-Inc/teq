// jars: fixtures scala-library
// teq: --werror
// targets: js interp jvm
// A library's constructor accessors qualified `private[shapes]` and `protected[shapes]` (tests/tasty/src/paramqual.scala,
// TASTy's `PRIVATEqualified` and `PROTECTEDqualified`, which a constructor's parameters keep as the index's members
// do): an `override val` inside the package is the private one's inherited accessor (RefChecks (1.12)'s
// `isAccessibleFrom`), and outside it a pattern, `copy` and a subclass's read of the protected one stay as scalac
// has them.
package fix.shapes {
  class PqB(s: Int) extends PqA(s)
  class PqInside(override val s: Int) extends PqB(s)
}

class PqH2(n: Int) extends fix.shapes.PqH(n):
  def get = h

object PqUse:
  def sum(k: fix.shapes.PqK) = k match
    case fix.shapes.PqK(a, b) => a + b
  def again(k: fix.shapes.PqK) = k.copy(y = 2)

@main def run(): Unit =
  println(fix.shapes.PqInside(1).s)
  println(PqUse.sum(fix.shapes.PqK(1, 2)))
  println(PqUse.again(fix.shapes.PqK(1, 5)).y)
  println(PqH2(3).get)
