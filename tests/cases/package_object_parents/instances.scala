package shop

trait Show[A]:
  def show(a: A): String

trait PriceInstances:
  implicit def showPrice: Show[Double] = new Show[Double]:
    def show(a: Double): String = s"$$$a"
  def label(s: String): String = s"[$s]"
  extension (d: Double) def cents: Int = (d * 100).toInt

trait Greeting:
  def prefix: String = "hello"
  def greet(who: String): String = s"$prefix, $who"
