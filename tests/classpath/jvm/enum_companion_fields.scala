// jars: scala-library
// std: scala-library
// An enum's values as its companion's static fields in scalac's layout: the program reads them there, the
// companion stores them before its body runs, which reads them;
// `values` copies `$values`; the companion is the enum's `Mirror.Sum`, whose `ordinal` its bridge answers.
import scala.deriving.Mirror
enum Color:
  case Red, Green, Blue
object Color:
  val first = Red
  val all = List(Red, Green, Blue)
  println(s"companion body sees $first and ${all.mkString(",")}")
enum Shape:
  case Dot
  case Box(w: Int)
  case Line
object Main:
  def main(args: Array[String]): Unit =
    println(Color.Green)
    println(Color.first)
    println(Color.values.mkString(","))
    val vs = Color.values
    vs(0) = Color.Blue
    println(Color.values.mkString(","))
    println(Color.valueOf("Blue").ordinal)
    println(Color.fromOrdinal(1))
    println(Shape.Line.ordinal)
    val m = summon[Mirror.SumOf[Color]]
    println(m.ordinal(Color.Blue))
    val c = (Color: Any).asInstanceOf[Mirror.SumOf[Color]]
    println(c.ordinal(Color.Green))
    def name(c: Color): String = c match
      case Color.Green => "green"
      case Color.Red => "red"
      case _ => "other"
    println(Color.values.map(name).mkString(","))
