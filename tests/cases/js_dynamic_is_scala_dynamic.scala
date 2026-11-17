//> using platform js
import scala.scalajs.js
import scala.reflect.ClassTag
trait Key[T]:
  def text: String
given Key[scala.Dynamic] with
  def text = "marker"
def keyed[T](using k: Key[T]): String = k.text
def marker(x: scala.Dynamic): String = "marker"
@main def run(): Unit =
  val d: js.Dynamic = js.Dynamic.literal(a = 1)
  val m: scala.Dynamic = d
  println(marker(d))
  println(keyed[scala.Dynamic])
  println(summon[ClassTag[scala.Dynamic]].runtimeClass.getName)
  val a: Any = d
  println(a.isInstanceOf[Dynamic])
  println(a match { case _: Dynamic => "marker"; case _ => "other" })
  println(d.a)
  println(js.Dynamic.literal(a = 1).a)
  println(js.Dynamic.global.Math.max(1, 2))
