// jars: scala-library
// `canEqual` on an `Equals` is scala-library's member, called virtually as scalac calls it, so a class's own answers
// (master: "found Equals, required Product", the call taken for teq's product helper); a class extending `Equals`
// takes `Any`'s `equals` for the trait's abstract redeclaration, as dotty's `RefChecks.isImplemented` does (master:
// "class K needs to be abstract"). JavaScript and the interpreter still answer a product's `canEqual` by its class.
case class C(x: Int)
class K extends Equals { def canEqual(that: Any) = that == 1 }
case class D(y: Int) { def canEqual(that: Any) = true }
def f(e: Equals, x: Any): Boolean = e.canEqual(x)
def g(p: Product, x: Any): Boolean = p.canEqual(x)
@main def run() =
  println(f(C(1), C(2)))
  println(f(C(1), "s"))
  println(f(new K, 1))
  println(f(new K, 2))
  println(g(D(1), "s"))
  println(f(D(1), "s"))
  println(g(C(1), C(3)))
