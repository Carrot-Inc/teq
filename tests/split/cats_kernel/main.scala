// jars: cats-kernel cats-core sourcecode
// The determinism of a library's bodies, read from cats' pickles and converted into the pseudo file whose line
// offsets name their anonymous classes (src/typer/loader/bodies.rs `Conv::line`), built at each worker count and
// compared: at M2's gate (4b0523a27) the application gate's frontend and API builds wrote different cats.mjs and
// cats.kernel.mjs from one build to the next, the anonymous classes' numbers off by one.
import cats.*
import cats.data.*
import cats.kernel.{Eq, Hash, Monoid, Order, Semigroup}
import cats.syntax.all.*
case class P(a: Int, b: String)
object P:
  given Eq[P] = Eq.by(p => (p.a, p.b))
  given Hash[P] = Hash.by(p => (p.a, p.b))
  given Order[P] = Order.by(p => (p.a, p.b))
  given Monoid[P] = Monoid.instance(P(0, ""), (x, y) => P(x.a + y.a, x.b + y.b))
@main def run(): Unit =
  val xs = List(P(1, "a"), P(2, "b"))
  println(xs.combineAll)
  println(xs.sorted(using Order[P].toOrdering))
  println(NonEmptyList.of(1, 2, 3).reduce)
  println(Validated.valid[String, Int](1).combine(Validated.valid(2)))
  println(Chain(1, 2).foldMap(_.toString))
  println(Hash[P].hash(P(1, "a")) == Hash[P].hash(P(1, "a")))
  println(xs.traverse(p => Option(p.a)))
  println(Map(1 -> "a").combine(Map(1 -> "b")))
