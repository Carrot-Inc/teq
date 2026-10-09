// jars: scala-library
// std: scala-library
// `ordinal` on a value bounded by `scala.reflect.Enum` in link mode: the std's trait stands for
// the jar's interface, so the call is that interface's method on the enum's class file.
enum Color:
  case Red, Green, Blue

def ordinalOf[E <: scala.reflect.Enum](e: E): Int = e.ordinal

object Main:
  def main(args: Array[String]): Unit =
    println(Color.values.map(ordinalOf).mkString(","))
    println(ordinalOf(Color.Blue) - ordinalOf(Color.Red))
