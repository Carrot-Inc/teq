package pca

// A polymorphic context function: `PolyFunction { def apply[A](using x$1: A): A }`, applied with a
// using clause and instantiated without one, which scalac eta-expands as a context function.
object P:
  type Context = [A] => A ?=> A
  val context: Context = [A] => (a: A) ?=> a
  def call(f: Context): Int = f[Int](using 7)
  def instantiate(f: Context): Int ?=> Int = f[Int]
  type Two = [A, B] => (A, B) => (B, A)
  val two: Two = [A, B] => (a: A, b: B) => (b, a)
  def inferred(f: Two): (String, Int) = f(7, "x")
  def expected(f: [A] => List[A] => List[A]): List[Int] = f(Nil)
