// The operand is bound ahead of a receiver whose evaluation can do something: a selection that throws on a null
// prefix, an object a subclass implements an abstract value with (its initializer runs where it is first read), a
// receiver whose own member does not take the operand and is converted instead, and, in an inline method's body, a
// parameter or `this` that an expansion makes a lazy value or an object with an initializer. Master evaluated
// each of those receivers first.
import scala.language.implicitConversions
object Log:
  var s: String = ""
  def hit[A](tag: String, a: A): A = { s += tag; a }
  def take(): String = { val r = s; s = ""; r }
def left(): Int = Log.hit("l", 1)
class P(val items: List[Int])
trait T { val box: Box }
class Box { def +:(x: Int): String = "box" + x }
class C extends T { object box extends Box { Log.hit("i", ()) } }
class Raw { def ++:(x: String): String = "raw" }
class Rich { def ++:(x: Int): String = "rich" + x }
given Conversion[Raw, Rich] = _ => Log.hit("c", new Rich)
lazy val lz: List[Int] = Log.hit("z", Nil)
inline def onList(xs: List[Int]): List[Int] = left() :: xs
object Loud { Log.hit("o", ()); def +:(x: Int): String = "loud" + x }
inline def onObj(r: Loud.type): String = left() +: r
object Loud2:
  Log.hit("o", ())
  def +:(x: Int): String = "loud" + x
  inline def pre(): String = left() +: this
@main def run(): Unit =
  val p: P = null
  try println(left() :: p.items)
  catch case _: Throwable => println(s"caught ${Log.take()}")
  val t: T = new C
  println(s"${left() +: t.box} ${Log.take()}")
  val raw = new Raw
  println(s"${left() ++: raw} ${Log.take()}")
  println(s"${onList(lz)} ${Log.take()}")
  println(s"${onObj(Loud)} ${Log.take()}")
  println(s"${Loud2.pre()} ${Log.take()}")
  val plain = List(0)
  println(s"${onList(plain)} ${Log.take()}")
