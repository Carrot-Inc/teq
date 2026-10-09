// A member selected on a union of two applications of one invariant class is the class's over
// `? >: A & B <: A | B`: writing through it takes what is both, as dotc's `lubArgs` gives, and
// a lambda over the argument (`map(e => ..)`) takes the wildcard's capture, whose lower bound
// is that intersection.
trait A:
  def a: String = "a"
trait B:
  def b: String = "b"
class AB extends A with B
class Cell[T]:
  var held: Option[T] = None
  def put(t: T): String =
    held = Some(t)
    "ok"
  def get: Option[T] = held

@main def run(): Unit =
  val c = if System.nanoTime() > 0 then new Cell[A] else new Cell[B]
  println(c.put(new AB))
  println(c.get.map(_.toString.length > 0))
  joins()

sealed trait E
case class H(x: Int) extends E
case class N(s: String) extends E
class Fr[A](val a: A):
  def map[B](f: A => B): Fr[B] = new Fr(f(a))

def joins(): Unit =
  val n = System.nanoTime() > 0
  val fr = if n then new Fr[H](H(1)) else new Fr[N](N("n"))
  println(fr.map(e => e.toString).a)
  println((if n then new Fr[H](H(2)) else new Fr[N](N("m"))).map(_ => 3).a)
