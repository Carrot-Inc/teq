// expect: 15:22: error: object creation impossible, since def size: Int in trait Named is not defined
// expect: 17:18: error: the type arguments of Functor cannot be inferred here; write them out
// expect: 21:37: error: type mismatch: found String, required Int
// expect: 3 errors found

trait Named:
  def name: String
  def size: Int
class Plain(val x: Int)
trait Functor[F[_]]:
  def map[A, B](fa: F[A])(f: A => B): F[B]

val bad: Plain = new Plain(1) { def y: Int = 2 }

val missing: Named = new Named { def name: String = "n" }

val noArgs = new Functor { def map[A, B](fa: List[A])(f: A => B): List[B] = fa.map(f) }

def counter(start: Int): Named = new Named:
  def name: String = "c"
  def size: Int = if start > 0 then "big" else 0

@main def run(): Unit = println(bad.x)
