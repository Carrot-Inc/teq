// A repeated argument that is a cast to a value class, passed where a universal trait of the class is expected: the
// argument's adaptation makes the cast over again, and so does the adaptation `addArg` makes after it (a tree of the
// same shape, not the same tree).
trait U extends Any:
  def size: Int
final class V(val i: Int) extends AnyVal with U:
  def size: Int = i
def consume(xs: U*): Int = xs.map(_.size).sum
def f(x: Any): Int = consume(x.asInstanceOf[V])
def g(x: Any, y: V): Int = consume(x.asInstanceOf[V], y, new V(3))

@main def run(): Unit =
  println(f(new V(1)))
  println(g(new V(1), new V(2)))
  println(consume())
