package pia

// A polymorphic function applied with its type argument inferred (`f(42)`, `f.apply[Int]`),
// a curried one whose later clause settles it, and one instantiated without arguments
// (`f[Int]`), which scalac eta-expands.
object P:
  type Id = [T] => T => T
  val id: Id = [T] => (x: T) => x
  def use(f: Id): Int = f(42)
  def both(f: Id): (String, Boolean) = (f("s"), f(true))
  def inst(f: Id): Int => Int = f[Int]
  // The instantiated function's receiver is evaluated once, where the expansion stands.
  var made = 0
  def make(): Id =
    made += 1
    id
  def instOnce: Int =
    val g = make()[Int]
    g(1) + g(2) + made * 100
  // A curried one, whose later clause settles the argument (`RunCTask`'s shape).
  type Run = [A] => Int => (Int => A) => A
  val run: Run = [A] => (n: Int) => (k: Int => A) => k(n)
  def label(r: Run): String = r(3)(n => "n" + n)
