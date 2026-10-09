// After Scala 3's tests/pos/i14187-like protected constructors, neg/targetName-override.scala's
// consistent target names, and a val next to a method of its name, which scalac keeps.
import scala.annotation.targetName

trait Show[A]:
  def show(a: A): String
trait Instances:
  given Show[Int] with
    def show(a: Int) = s"int $a"
  implicit def showStr: Show[String] = new Show[String]:
    def show(a: String) = s"str $a"

class Counter:
  val count: Int = 3
  def count(step: Int): Int = count + step

class User extends Instances:
  def render[A](a: A)(using s: Show[A]): String = s.show(a)
  def both: String = render(1) + " " + render("x")

class Box protected (val v: Int):
  def bigger: Box = new Box(v + 1)
object Box:
  def make(v: Int) = new Box(v)
class Big(v: Int) extends Box(v)

abstract class Alpha[T]:
  def foo() = 1
  @targetName("foo1") def foo(x: T): T
  @targetName("append") def ++(xs: Alpha[T]): Alpha[T] = this
class Beta extends Alpha[String]:
  @targetName("foo1") def foo(x: String): String = x + x
  @targetName("append") override def ++(xs: Alpha[String]): Alpha[String] = xs

object Main:
  val b = 2
  def main(args: Array[String]): Unit =
    val c = new Counter
    println(c.count)
    println(c.count(4))
    println(new User().both)
    println(Box.make(5).v)
    println(new Big(6).bigger.v)
    println(2 match { case `b` => "b"; case _ => "other" })
    println(3 match { case `b` => "b"; case _ => "other" })
    val beta = new Beta
    println(beta.foo("a") + beta.foo())
    println((beta ++ beta) eq beta)
