//> using platform jvm
// `A & Pure` and `Pure & A` erase as `A` (TypeErasure.scala:801-804), `A`'s class symbol read through its aliases but
// not through a match case's bounds: casts and abstract type tests check `Z`, an array of `Z & Pure` is `Z[]`, an
// opaque alias of `Pure` is `Pure` in its own scope, and a `T` that a case fixes to `Pure` keeps its declared bound.
import scala.reflect.ClassTag
import scala.caps.Pure
trait Z
class ZI extends Z
class ZP extends Z with Pure
trait H:
  type A <: Z & Pure
  type B <: Pure & Z
  def ta(x: Any): String = x match { case _: A => "A"; case _ => "no" }
  def tb(x: Any): String = x match { case _: B => "B"; case _ => "no" }
object HI extends H:
  type A = ZP
  type B = ZP
object O:
  opaque type OP = Pure
  def castIn(a: Any): String = Main.cc(a.asInstanceOf[OP & Z])
  def mIn[A <: OP & Z](x: Any): String = x match { case _: A => "A"; case _ => "no" }
enum E[T]:
  case P extends E[Pure]
  case Q extends E[Int]
object Main:
  def t[T](using c: ClassTag[T]): String = c.runtimeClass.getName
  def cc(f: => Any): String = try { f; "cast" } catch { case _: ClassCastException => "cce" }
  def castT[T <: Pure](a: Any): String = cc(a.asInstanceOf[T & Z])
  def m[A <: Z & Pure](x: Any): String = x match { case _: A => "A"; case _ => "no" }
  def m2[A <: Pure & Z](x: Any): String = x match { case _: A => "A"; case _ => "no" }
  def g[T](e: E[T], a: Any): String = e match
    case E.P => cc(a.asInstanceOf[T & Z]) + " " + cc(a.asInstanceOf[Z & T])
    case E.Q => "q"
  def main(args: Array[String]): Unit =
    val (o, zi) = (new Object, new ZI)
    println(t[Array[Z & Pure]])
    println(List(cc(o.asInstanceOf[Z & Pure]), cc(zi.asInstanceOf[Z & Pure]), cc(o.asInstanceOf[Pure & Z]), castT[Pure](o), castT[Pure](zi)).mkString(" "))
    println(List(m[ZP](o), m[ZP](zi), m2[ZP](o), m2[ZP](zi), HI.ta(o), HI.ta(zi), HI.tb(o), HI.tb(zi)).mkString(" "))
    println(List(O.castIn(o), O.mIn[Nothing](o), g(E.P, o)).mkString(" "))
