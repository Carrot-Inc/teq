package tpa

// Generic traits with parameters named without type arguments in parent clauses: the type
// arguments inferred from the arguments, as scalac's typer types the parent's application, for
// a primitive and a reference instantiation, a class's own type parameter, a trait after a class
// parent, two such traits in one class and a lambda, which its class is made for once; and a
// parameter left to its default, which a build over the products leaves to its default too.
trait Plain[A](val first: A):
  def twice: List[A] = List(first, first)

trait Other[B](val second: B)

class Base(val n: Int)

class P extends Plain(3)
object O extends Plain("s")
class Q[C](c: C) extends Plain(c)
class R extends Base(1), Plain("r")
class Two extends Plain(2L), Other(true)
object L extends Plain((x: Int) => x + 1)

trait Defaulted[A](val a: A, val b: Int = 7)
class D extends Defaulted("d")
class Base2(val x: Int, val y: Int = 3)
class E extends Base2(1)
