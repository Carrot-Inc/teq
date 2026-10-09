// A cast of `()` to a trait, a class, `String` or `Null` fails with a ClassCastException, the
// operand run once: on JavaScript `()` is `undefined`, which the trait's test fails as a type
// test of it does. `null` passes a cast to a trait.
trait T
class A
var n = 0
def unit: Unit = { n += 1; () }
def attempt(label: String)(f: => Any): Unit =
  try { f; println(label + " ok") }
  catch
    case _: ClassCastException => println(label + " CCE")
    case _: Throwable => println(label + " other")
@main def run(): Unit =
  val x: Any = ()
  attempt("Any Unit to trait") { x.asInstanceOf[T]; () }
  attempt("Unit to trait") { unit.asInstanceOf[T]; () }; println(n)
  attempt("Unit to class") { unit.asInstanceOf[A]; () }; println(n)
  attempt("Unit to String") { unit.asInstanceOf[String]; () }; println(n)
  attempt("Unit to Null") { unit.asInstanceOf[Null]; () }; println(n)
  attempt("used") { val t: T = x.asInstanceOf[T]; println(t) }
  attempt("null trait") { (null: Any).asInstanceOf[T]; () }
