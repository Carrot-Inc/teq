// jars: scala-library scala-java-time scala-java-time-tzdb
// Each value of an enum that extends `java.lang.Enum` is a `Mirror.Singleton` (dotty's `DesugarEnums` attaches
// `ExtendsSingletonMirror` to every value's class, whatever the enum extends): the sixteen of scala-java-time's
// `ChronoUnit`, read from its jar, whose classes scalac wrote with `scala.deriving.Mirror$Singleton` among their
// interfaces, and the three of an enum of the program's. scalac 3.8.4 prints 16 and 3.
import scala.deriving.Mirror
import java.time.temporal.ChronoUnit

enum Level(val rank: Int) extends java.lang.Enum[Level]:
  case Low extends Level(1)
  case Mid extends Level(2)
  case High extends Level(3)

object Units:
  def singles(xs: List[Any]): Int = xs.count(_.isInstanceOf[Mirror.Singleton])
  def main(args: Array[String]): Unit =
    println(singles(ChronoUnit.values.toList))
    println(singles(Level.values.toList))
