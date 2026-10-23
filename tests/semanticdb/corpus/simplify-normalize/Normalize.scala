class Box[A]
object Normalize {
  type M[X] = X match { case Int => String; case _ => Boolean }
  def bounded[A]: Box[? <: M[A]] = ???
  val result = bounded[Int]
  def poly[A]: [X] => X => M[A] = ???
  val resultPoly = poly[Int]
  def refined[A]: { def f(x: A): M[A] } = ???
  val resultRefined = refined[Int]
}
