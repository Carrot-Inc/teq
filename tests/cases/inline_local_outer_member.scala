// A local inline method of an inner class's method that calls its outer class's members: its expansion reaches them
// through the outer instance, `Inner.this`'s outer (`Inliner.computeThisBindings`' `inlinedMethod.owner`).
trait Measures {
  def ignored(x: Int): Boolean = x == 0
  protected final def reduced(x: Int): Int = if ignored(x) then 0 else x % 10
}

class Outer(val base: Int) extends Measures {
  def shifted(x: Int): Int = x + base
  final class Inner(depth: Int) {
    def check(x: Int): Int = {
      inline def prepared(y: Int): Int = reduced(shifted(y))
      prepared(x + depth)
    }
    def checkExt(x: Int): Int = {
      extension (y: Int) inline def prep: Int = reduced(y) + base
      (x + depth).prep
    }
  }
  def run(x: Int): Int = new Inner(1).check(x)
  def runExt(x: Int): Int = new Inner(1).checkExt(x)
}

@main def run(): Unit =
  val o = new Outer(3)
  println(o.run(0))
  println(o.run(21))
  println(o.runExt(0))
  println(o.runExt(-1))
