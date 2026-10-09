def side(msg: String): Int = { println("eval " + msg); msg.length }
class Base(a: Int):
  println("Base body " + a)
trait First(val f: Int):
  println("First body " + f)
  val doubled = f * 2
trait Second(s: Int, val label: String = "dflt"):
  println("Second body " + s + " " + label)
  def total = s + 1
trait Plain:
  println("Plain body")
  val p = 5
  var counter = 0
  lazy val late = { println("late!"); 9 }
class Impl(x: Int) extends Base(side("base")) with First(side("first!") + x) with Plain with Second(side("second!!")):
  println("Impl body " + doubled + " " + total + " " + p)
object Obj extends First(3) with Second(4, "obj")
@main def run(): Unit =
  val i = Impl(100)
  println(i.f + " " + i.label + " " + i.late + " " + i.late)
  i.counter += 2
  println(i.counter)
  println(Obj.doubled + Obj.total)
  val a = new First(7) with Plain { println("anon " + (doubled + p)) }
  println(a.f)
