// A plain inline method named without arguments (cats-mtl's `allow[E]: AdHocSyntaxWired[E]`) is kept
// by the typer for the inlining phase and typed as an ordinary call (`Inlines.needsInlining` false):
// its type variable stays open for the trees around, `E` taken from the body applied to it (the
// handler's `err` a `String`, not `Any`); one its type does not name takes its upper bound, as the
// interpolation of the call's tree gives it (`Inferencing.instantiateTypeVars`), the expansion's
// type argument (`named` prints `Any`); a covariant one takes its lower bound.
trait Raise[-E]:
  def raise(e: E): Nothing

final class Allow[E]:
  def apply[A](body: Raise[E] ?=> A): Inner[E, A] = Inner(body)

class Boom(val e: Any) extends Exception

final class Inner[E, A](body: Raise[E] ?=> A):
  def rescue(h: E => A): A =
    try body(using e => throw Boom(e))
    catch case b: Boom => h(b.e.asInstanceOf[E])

inline def allow[E]: Allow[E] = Allow[E]

def parse(s: String)(using r: Raise[String]): Int =
  s.toIntOption.getOrElse(r.raise(s"not a number: $s"))

def parseAll(xs: List[String]): (List[Int], Option[String]) =
  allow:
    (xs.map(parse), None)
  .rescue: err =>
    (Nil, Some(err.toUpperCase))

class Box[A](val a: List[A]):
  def add(x: A): Box[A] = Box(x :: a)
inline def box[A]: Box[A] = Box[A](Nil)
inline def none[A]: Option[A] = None
inline def meth[R <: Int]: R = 3.asInstanceOf[R]
inline def named[A]: String = compiletime.summonInline[reflect.ClassTag[A]].toString

@main def run(): Unit =
  println(parseAll(List("1", "2")))
  println(parseAll(List("1", "x")))
  val b: Box[Int] = box.add(1)
  println(b.a)
  println(box.add("x").add("y").a)
  val s: Option[String] = none
  println(s)
  val m: Int = meth
  println(m)
  println(named)
