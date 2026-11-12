// `Object` among a class's base classes decides a union with `Null` (`erasedLub`): a value class over `Ordered` derives
// from it through `Comparable`, a plain one does not; a universal trait over universal traits (`IterableOnce`,
// `Selectable`, `SeqOps`, `Factory`, each `extends Any` as the library declares it) does not, so its union erases to
// `Object` and its cast checks nothing.
import scala.reflect.ClassTag
class Meters(val v: Int) extends AnyVal with Ordered[Meters]:
  def compare(o: Meters): Int = v - o.v
class VP(val v: Int) extends AnyVal
trait UIO extends Any with IterableOnce[Int]
trait USel extends Any with Selectable
trait USeq extends Any with collection.SeqOps[Int, List, List[Int]]
trait UFac extends Any with collection.Factory[Int, List[Int]]
object Main:
  def t[T](using c: ClassTag[T]): String = c.toString
  def cce(f: => Any): String = try { f; "cast" } catch { case _: ClassCastException => "CCE" }
  def main(args: Array[String]): Unit =
    println(t[Meters | Null] + " " + t[VP | Null])
    println(List(t[UIO | Null], t[USel | Null], t[USeq | Null], t[UFac | Null]).mkString(" "))
    println(List(cce("s".asInstanceOf[UIO | Null]), cce("s".asInstanceOf[USel | Null]), cce("s".asInstanceOf[USeq | Null]), cce("s".asInstanceOf[UFac | Null])).mkString(" "))
