// Abstract polymorphic values keep their argument lists after apply insertion.
trait PolyFactory:
  type F <: [A] => A => A => A
  val function: F
  def fresh = function

@main def refinedPolyApplications(): Unit =
  val factory = new PolyFactory:
    type F = [A] => A => A => A
    val function = [A] => (first: A) => (second: A) => second
  println(factory.fresh(1)(2))
  println(factory.fresh("first")("second"))
