// A literal or constant pattern stands for its constant of its own type, adapted to a numeric scrutinee's, as
// dotty's typer adapts it (`case 1` of a `Short` is `1: Short`, `case Byte.MinValue` of a `Long` is `-128L`):
// none of these cases is unreachable. scalac prints the lines of the .expected file and warns nothing.
def a(s: Short): String = s match { case java.lang.Short.MAX_VALUE => "smax"; case _ => "other" }
def b(s: Short): String = s match { case Short.MaxValue => "smax"; case _ => "other" }
def c(x: Byte): String = x match { case java.lang.Byte.MAX_VALUE => "bmax"; case java.lang.Byte.MIN_VALUE => "bmin"; case _ => "other" }
def g(x: Long): String = x match { case Byte.MinValue => "bmin"; case _ => "other" }
def h(x: Short): String = x match { case 1 => "one"; case 2 => "two"; case _ => "other" }
def i(x: Byte): String = x match { case 1 => "one"; case -1 => "minus one"; case _ => "other" }
def j(x: Float): String = x match { case 1.5f => "f"; case _ => "other" }
def k(x: Int): String = x match { case java.lang.Short.MAX_VALUE => "smax"; case _ => "other" }
def l(x: Char): String = x match { case 65 => "A"; case _ => "other" }
@main def run(): Unit =
  println(List(a(32767), b(32767), c(127), c(-128), g(-128L), h(2), i(-1), j(1.5f), k(32767), l('A')))
