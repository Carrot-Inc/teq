// A case class's application is its companion's `apply`, whose qualifier is the companion: where the companion has
// initialisation the argument is no pure expression (dotty's `TreeInfo.refPurity` of a module), so `typedInfixOp`'s
// lift binds it ahead of the application and the companion's initializer runs before the operand on the right; a
// companion without initialisation leaves the creation pure and unbound. The creation counts wherever it stands in
// the argument: an element of a creation, of a tuple.
case class A(n: Int)
object A:
  println("init A")
case class B(n: Int)
object B:
  val made = { println("init B"); 0 }
case class D(n: Int)
object D:
  println("init D")
case class G[T](t: T)
object G:
  println("init G")
case class N(n: Int)
object N:
  def zero: N = N(0)
class Box:
  def +:(x: Any): String = "box"
def right(s: String): List[Any] = { println("right " + s); Nil }
def rbox(): Box = { println("right box"); new Box }

@main def main(): Unit =
  println(A(1) :: right("a"))
  println(Some(B(1)) :: right("b"))
  println((D(1), 2) :: right("d"))
  println(G("g") +: rbox())
  println(N(1) :: right("n"))
