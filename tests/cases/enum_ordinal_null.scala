//> using platform jvm
// `ordinal` through `scala.reflect.Enum` on `null` fails as any call on `null` does
// (`NullPointerException`), the receiver's implementation or an enum case's alike; on JavaScript
// a member selected from `null` is the engine's `TypeError` (docs/COMPATIBILITY.md).
case class Manual(x: Int) extends scala.reflect.Enum:
  def ordinal: Int = 42

enum Color:
  case Red, Green

object Main:
  def main(args: Array[String]): Unit =
    val e: scala.reflect.Enum = Manual(1)
    println(e.ordinal)
    val absent: scala.reflect.Enum = null
    try println(absent.ordinal)
    catch case _: NullPointerException => println("NPE")
    val none: Color = null
    try println(none.ordinal)
    catch case _: NullPointerException => println("NPE")
