// Casts and abstract type tests erase at the erasure phase, after ElimOpaque has made every opaque alias a plain
// alias (Erasure.scala:773-777), so an opaque alias of `Pure` is `Pure` there whatever its prefix, and `O.OP & Z`
// checks `Z` (TypeErasure.scala:801-804). The ClassTag synthesis reads the same types in the typer, where `O.OP`
// outside its scope, or through its static prefix, is abstract and erases to `Object`.
import scala.caps.Pure
import scala.reflect.ClassTag
trait Z
class ZI extends Z
object O:
  opaque type OP = Pure
  opaque type OPB <: Pure = Pure
  opaque type OU = Pure | Null
  def inside(o: Any): String =
    List(Main.cc(o.asInstanceOf[O.OP & Z]), Main.cc(o.asInstanceOf[OP & Z]), Main.cc(o.asInstanceOf[this.OP & Z]),
      Main.m[Nothing](o), Main.tag[O.OP & Z], Main.tag[OP & Z]).mkString(" ")
object Main:
  def cc(f: => Any): String = try { f; "cast" } catch { case _: ClassCastException => "cce" }
  def tag[T](using c: ClassTag[T]): String = c.runtimeClass.getName
  def m[A <: O.OP & Z](x: Any): String = x match { case _: A @unchecked => "A"; case _ => "no" }
  def mu[A <: O.OU & Z](x: Any): String = x match { case _: A @unchecked => "A"; case _ => "no" }
  def main(args: Array[String]): Unit =
    val o = new Object
    val zi = new ZI
    println(O.inside(o))
    println(List(cc(o.asInstanceOf[O.OP & Z]), cc(o.asInstanceOf[Z & O.OP]), cc(zi.asInstanceOf[O.OP & Z]),
      cc(o.asInstanceOf[O.OPB & Z]), cc(o.asInstanceOf[O.OU & Z])).mkString(" "))
    println(List(m[Nothing](o), m[Nothing](zi), mu[Nothing](o), mu[Nothing](zi)).mkString(" "))
    println(List(tag[O.OP & Z], tag[O.OPB & Z], tag[O.OU & Z]).mkString(" "))
