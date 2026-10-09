// jars: scala-library
// std: scala-library
// The getters of defaults in scalac's layout: a method's take
// the parameters of the clauses before the default's, a by-name parameter's returns the value and the caller
// passes a thunk calling it, evaluated each time the parameter is read; a constructor's are its companion's,
// written by the backend for a plain class (`Plain$`), a secondary constructor's too.
var count = 0
def tick(): Int = { count += 1; count * 10 }
object O:
  def f(x: Int = 1, y: => Int = tick()): Int = x + y + y
  def curried[A](a: A)(b: String = a.toString)(c: Int = b.length): Int = c
  def scaled(x: Int, y: Int = 2): Int = x * y
class D:
  def scaled(x: Int, y: Int = 2): Int = x * y
  def byName(z: => String = "z" + tick()): String = z + z
class Plain(val a: Int = 1)(val b: String = a.toString + "!")
case class CC(a: Int = 3)(val b: String = (a * 2).toString)
class Sec(val a: Int, val b: Int):
  def this(a: Int, s: String = "7") = this(a, s.toInt)
object Plain2:
  def make = new Plain()()
@main def run(): Unit =
  println(O.f())
  println(count)
  println(O.f(5))
  println(count)
  println(O.curried(42)()())
  println(O.curried(42)("ab")())
  println(O.scaled(4))
  println(new D().scaled(5))
  println(new D().byName())
  val p = new Plain()()
  println(s"${p.a} ${p.b}")
  val q = new Plain(9)()
  println(s"${q.a} ${q.b}")
  val c = CC()()
  println(s"$c ${c.b}")
  val s = new Sec(1, "8")
  println(s"${s.a} ${s.b}")
  val s2 = new Sec(2)
  println(s"${s2.a} ${s2.b}")
  println(Plain2.make.b)
