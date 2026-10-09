// Same-named extension methods inside one object or class: they differ by receiver type or
// arity, so each needs its own method in the output.

final class AttrKey(val name: String)
final class StyleKey(val name: String)
final class EventKey(val name: String)

object Dsl:
  extension (key: AttrKey)
    def :=(value: String): String = key.name + "=" + value
  extension (key: StyleKey)
    def :=(values: String*): String = key.name + "=" + values.toList.mkString(";")
  extension (key: EventKey)
    def :=(handler: Int => Int): String = key.name + "=" + handler(20).toString

  def describe(n: Int): String = "member " + n.toString
  extension (s: String)
    def describe: String = "string " + s
    def repeat(times: Int): String = s * times
    def repeat(times: Int, sep: String): String = List.fill(times)(s).mkString(sep)
  extension (b: Boolean)
    def describe: String = "boolean " + b.toString

trait Show[A]:
  extension (a: A) def show: String

given Show[Int] with
  extension (a: Int) def show: String = "int " + a.toString

final class Printer(prefix: String):
  extension (n: Int) def out: String = prefix + n.toString
  extension (s: String) def out: String = prefix + s
  def both: String = 1.out + " " + "one".out

def shown[A: Show](a: A): String = a.show

import Dsl.*

@main def run(): Unit =
  println(AttrKey("href") := "/home")
  println(StyleKey("style") := ("a", "b"))
  println(StyleKey("style") := "solo")
  println(EventKey("click") := (_ + 1))
  println(Dsl.describe(3))
  println("x".describe)
  println("x".repeat(3))
  println("x".repeat(3, "-"))
  println(true.describe)
  println(shown(7))
  println(7.show)
  println(Printer("> ").both)
