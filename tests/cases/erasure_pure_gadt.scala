//> using platform jvm
// A GADT case fixes `T` without changing what `T & zw` erases as: `T`'s declared bound (`T <: Pure`) makes it `zw`,
// and a case that knows `T = Pure` leaves an unbounded `T` erasing to `Object` (TypeErasure.scala:801-804).
import scala.caps.Pure
import scala.reflect.ClassTag
trait zw
class WI extends zw
class PI extends Pure
class WP extends zw with Pure
enum E[T]:
  case P extends E[Pure]
  case W extends E[WP]
  case Q extends E[Int]
object Main:
  def cc(f: => Any): String = try { f; "cast" } catch { case _: ClassCastException => "cce" }
  def inv[T <: Pure](e: E[T], a: Any): String = e match
    case E.W => cc(a.asInstanceOf[T & zw])
    case _ => "other"
  def invR[T <: Pure](e: E[T], a: Any): String = e match
    case E.W => cc(a.asInstanceOf[zw & T])
    case _ => "other"
  def gA[T, A <: T & zw](e: E[T], a: Any): String = e match
    case E.P => a match { case _: A => "A"; case _ => "no" }
    case _ => "other"
  def gAinv[T <: Pure, A <: T & zw](e: E[T], a: Any): String = e match
    case E.W => a match { case _: A => "A"; case _ => "no" }
    case _ => "other"
  def gTag[T](e: E[T])(using ct: ClassTag[T]): String = e match
    case E.P => summon[ClassTag[T]].runtimeClass.getName
    case _ => "other"
  def main(args: Array[String]): Unit =
    val objs = List(new Object, new WI, new PI, new WP)
    def row(n: String, f: Any => String) = println(n + " " + objs.map(f).mkString(" "))
    row("inv", inv(E.W, _))
    row("invR", invR(E.W, _))
    row("gA", gA[Pure, Nothing](E.P, _))
    row("gAinv", gAinv[WP, Nothing](E.W, _))
