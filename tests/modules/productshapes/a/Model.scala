package productshapes

case class C[A](a: A)(val b: A)
def copy[A](c: C[A], a: A): A => C[A] = c.copy(a = a)
def twice[A](a: => A): (A, A) = (a, a)
def lazyPair[A](a: A): () => A =
  lazy val (x, (y, z)) = { println("init"); (a, (1, 2)) }
  () => x
def lazySum(): Int =
  lazy val (p, q) = { println("sum"); (1, 2) }
  println("first")
  p + q
def cases: (Int, Int) => Int =
  val `x$case` = 10
  { case (a, b) => a + b + `x$case` }
def poly = [F[_]] => (x: F[Int]) => x
type Apply = [F[G[H[_]]]] =>> F[[K[_]] =>> K[Int]]
class Wrap[G[H[_]]](val x: G[List])
def wrap: Apply[Wrap] = new Wrap[[K[_]] =>> K[Int]](List(8))
class Outer(val base: Int):
  case class N(a: Int)(val b: Int):
    def total = base + a + b
    def bump = copy(a = a + 1)(b)
def again(o: Outer, n: o.N): Int = n.copy(a = 7)(8).total
