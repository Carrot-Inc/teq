// A repeated parameter's completed argument is its sequence, which `Typer.typedInfixOp` binds whole: every element
// is evaluated ahead of the receiver, a pure one that throws too (`box.value +: rep` with a null box throws before
// `rep`), and in an inline method's body the elements stay in their order where an expansion takes the bindings
// back beside a receiver that has become a value.
object Log:
  var s: String = ""
  def hit[A](tag: String, a: A): A = { s += tag; a }
  def take(): String = { val r = s; s = ""; r }
def a(): Int = Log.hit("a", 1)
def b(): Int = Log.hit("b", 2)
def c(): Int = Log.hit("c", 3)
class Rec:
  def +:(x: Int, ys: Int*): String = s"$x ${ys.mkString(",")}"
  def ++:(x: Int, y: Long): String = s"$x $y"
def mk(): Rec = Log.hit("r", new Rec)
inline def viaInline(inline r: Rec): String = (a(), b(), c()) +: r
inline def viaInline2(inline r: Rec): String = (a(), b()) ++: r
class Box(val value: Int)
class Rep { def +:(xs: Int*): Int = xs.sum }
def rep: Rep = { println("rep"); new Rep }
def loud: Int = { println("a"); 5 }
@main def run(): Unit =
  val rec = new Rec
  println(s"${(a(), b(), c()) +: mk()} ${Log.take()}")
  println(s"${viaInline(mk())} ${Log.take()}")
  println(s"${viaInline(rec)} ${Log.take()}")
  println(s"${viaInline2(mk())} ${Log.take()}")
  println(s"${viaInline2(rec)} ${Log.take()}")
  val box: Box = null
  try println(box.value +: rep)
  catch case _: Throwable => println("caught")
  try println((box.value, loud) +: rep)
  catch case _: Throwable => println("caught")
