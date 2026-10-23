package r01
class A1 extends scala.annotation.StaticAnnotation
class A2 extends scala.annotation.StaticAnnotation
@A1 @A2 class C:
  @A2 @A1 def f(@A1 @A2 x: Int) = x
  def g(using Ordering[Int]) = 1
  val h = g
object Exp:
  export Holder.{v, w as ww}
object Holder:
  val v = 1
  def w = 2
