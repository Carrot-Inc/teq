// expected: by hand
// A value of an intersection of value classes upcast to the side it does not erase to: `val s: VS = x` and an
// argument `p(x)` for `x: VS & VI`, whose value is `VI`'s (the first of the two by scalac's order), and the same
// with `VI` written first. teq's JVM throws a `ClassCastException` at the upcast, as scalac's explicit cast
// `x.asInstanceOf[VS]` does. scalac 3.8.4's classes for the upcasts fail the verifier (and Scala.js's back end
// crashes on them), so the expected lines are written by hand: what the program means, the cast's answer.
final class VI(val i: Int) extends AnyVal
final class VS(val s: String) extends AnyVal
def p(s: VS): String = s.s
object Up:
  def local(x: VS & VI): String = { val s: VS = x; s.s }
  def argument(x: VS & VI): String = p(x)
  def written(x: VI & VS): String = { val s: VS = x; s.s }
  def cast(x: VS & VI): String = x.asInstanceOf[VS].s
def t(label: String)(body: => String): Unit =
  try println(s"$label: $body") catch case e: ClassCastException => println(s"$label: ${e.getClass.getName}")
@main def run(): Unit =
  val x = new VI(3).asInstanceOf[VS & VI]
  t("a local")(Up.local(x))
  t("an argument")(Up.argument(x))
  t("a local, VI written first")(Up.written(new VI(4).asInstanceOf[VI & VS]))
  t("a cast")(Up.cast(x))
