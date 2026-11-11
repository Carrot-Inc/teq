// A non-native JS class's public and protected members are properties JavaScript reads by name,
// whether or not Scala code calls them, and a subclass's override wins; a private member no other
// member calls is none.
import scala.scalajs.js
class Box(n: Int) extends js.Object:
  val label: String = "box" + n
  var count: Int = n
  def value: Int = n
  def twice: Int = n * 2
  def plus(k: Int): Int = n + k
  private def secret: Int = 7
  protected def guarded: Int = 9
class Big(n: Int) extends Box(n):
  override def value: Int = n * 10
  def extra: String = "x"
@main def run(): Unit =
  val d = new Box(3).asInstanceOf[js.Dynamic]
  println(d.value)
  println(d.twice)
  println(d.plus(4))
  println(d.label)
  println(d.count)
  println(js.isUndefined(d.secret))
  println(d.guarded)
  val b = new Big(2).asInstanceOf[js.Dynamic]
  println(b.value)
  println(b.twice)
  println(b.extra)
