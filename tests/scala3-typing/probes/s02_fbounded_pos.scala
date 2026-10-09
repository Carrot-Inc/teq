trait Ord[T <: Ord[T]]:
  def cmp(o: T): Int
class X(val n: Int) extends Ord[X]:
  def cmp(o: X): Int = n - o.n
def max[T <: Ord[T]](a: T, b: T): T = if a.cmp(b) >= 0 then a else b
@main def run(): Unit = println(max(X(1), X(2)).n)
