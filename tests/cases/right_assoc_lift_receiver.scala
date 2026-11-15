// The left operand of a right-associative operator is bound ahead of the application where it is impure
// (`Typer.typedInfixOp`, `LiftImpure.liftArgs`), whatever today's test of the receiver said: a lazy value, a `var`,
// a `def`, an object with an initializer, a by-name parameter, a converted receiver. Where the completed receiver
// is a pure path (a value, a parameter, `this`, an object without an initializer) nothing can be observed between
// the two operands and no binding is written. Each statement logs both operands' effects.
import scala.language.implicitConversions
object Log:
  var s: String = ""
  def hit[A](tag: String, a: A): A = { s += tag; a }
  def take(): String = { val r = s; s = ""; r }
def left(): Int = Log.hit("l", 1)
object Quiet:
  def +:(x: Int): String = "quiet" + x
object Loud:
  Log.hit("o", ())
  def +:(x: Int): String = "loud" + x
class Wrapped
class Target:
  def +:(x: Int): String = "target" + x
given Conversion[Wrapped, Target] = w => Log.hit("c", new Target)
lazy val topLazy: List[Int] = Log.hit("r", Nil)
class Holder(val items: List[Int]):
  def +:(x: Int): String = "holder" + x
  def viaThis: String = left() +: this
  def viaField: List[Int] = left() :: items
def viaParameter(xs: List[Int]): List[Int] = left() :: xs
def viaByName(xs: => List[Int]): List[Int] = left() :: xs
def made(): List[Int] = Log.hit("r", Nil)
@main def run(): Unit =
  val value: List[Int] = List(0)
  println(s"${left() :: value} ${Log.take()}")
  println(s"${viaParameter(value)} ${Log.take()}")
  val holder = new Holder(value)
  println(s"${holder.viaThis} ${Log.take()}")
  println(s"${holder.viaField} ${Log.take()}")
  println(s"${left() :: Nil} ${Log.take()}")
  println(s"${left() +: Quiet} ${Log.take()}")
  println(s"${left() +: Loud} ${Log.take()}")
  println(s"${left() :: topLazy} ${Log.take()}")
  lazy val localLazy: List[Int] = Log.hit("r", Nil)
  println(s"${left() :: localLazy} ${Log.take()}")
  var changed: List[Int] = Nil
  def change(): Int = { changed = List(9); Log.hit("l", 1) }
  println(s"${change() :: changed} ${Log.take()}")
  println(s"${left() :: made()} ${Log.take()}")
  println(s"${viaByName(Log.hit("r", Nil))} ${Log.take()}")
  val wrapped = new Wrapped
  println(s"${left() +: wrapped} ${Log.take()}")
