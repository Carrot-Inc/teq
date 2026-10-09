package gco

// Given classes and given objects of a class, a trait and an enum whose bodies read their enclosing
// instance: each is an inner class holding that instance (scalac's `ExplicitOuter`), made with
// the receiver of the given's call, a mixing object's override read through it; called by its
// name inside the trait, a lambda, a nested class and a subclass, whose pickle calls it on `this`.
trait Show[A]:
  def show(a: A): String

trait Base:
  def tag: String = "base"
  given sized(using n: Int): Show[String] with
    def show(a: String): String = s"$tag:${a.take(n)}"
  given Show[Int] with
    def show(a: Int): String = s"$tag#$a"
  def inside: String = sized(using 1).show("abc")
  def lifted: List[String] = List(1, 2).map(k => sized(using k).show("abc"))
  class Nested:
    def deep: String = sized(using 3).show("abcd")

class Scale(val factor: Int):
  given scaled(using off: Int): Show[Long] with
    def show(a: Long): String = (a * factor + off).toString
  given Show[Boolean] with
    def show(a: Boolean): String = if a then s"yes*$factor" else s"no*$factor"

object Plain extends Base
object Loud extends Base:
  override def tag: String = "LOUD"
class Sub extends Base:
  override def tag: String = "sub"
  def viaThis: String = sized(using 2).show("xyz")

enum Tier(val base: Int):
  case Low extends Tier(10)
  case High extends Tier(20)
  given tiered(using i: Int): Show[Int] with
    def show(a: Int): String = s"${base + i + a}"
  given Show[Char] with
    def show(a: Char): String = s"$a$base"
