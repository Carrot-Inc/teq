// A trait's private is called as itself (Scala.js's `JSCodeGen.genApplyMethodMaybeStatically`), not
// the member of the same name of a class mixing the trait in.
trait S:
  private def h: Int = 3
  def k: Int = h
class C extends S:
  def h: Int = 4
class A:
  private def f: Int = 1
  def g: Int = f
class B extends A:
  def f: Int = 2
@main def run(): Unit =
  println(C().k)
  println(C().h)
  println(B().g)
  println(B().f)
