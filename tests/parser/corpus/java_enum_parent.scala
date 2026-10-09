// A Scala 3 enum over `java.lang.Enum`, whose cases carry their name and ordinal: `name`,
// `ordinal`, `toString`, `compareTo`, equality and `values`.
enum Color extends java.lang.Enum[Color]:
  case Red, Green, Blue
enum Span(val seconds: Long) extends java.lang.Enum[Span]:
  case Seconds extends Span(1L)
  case Minutes extends Span(60L)
object Main:
  def main(args: Array[String]): Unit =
    println(Color.Green.name() + " " + Color.Green.ordinal + " " + Color.Green + " " + Color.Red.compareTo(Color.Green) + " " + (Color.Red == Color.Red) + " " + Color.values.toList)
    println(Span.Minutes.name() + " " + Span.Minutes.seconds + " " + Span.Minutes.compareTo(Span.Seconds) + " " + Span.valueOf("Seconds").seconds)
