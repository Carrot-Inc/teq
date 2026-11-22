// A value of an intersection of value classes is boxed as the class the intersection erases to: scalac's
// `erasedGlb` of their erased value types, here `Feet` wherever it is written (classes nothing else orders go by their
// full names). So is it where it becomes an `Any`, an element, an option's value, a receiver of `toString`, and the
// `MatchError` of a match over it. The JVM boxed it as the value class written first, its `Int` converted to that
// class's underlying type (a `Meters` of a `Double`, a `Grams` of a `Long`), and a parameter of `Meters & Feet & Grams`
// was a `long` (`Grams`, the first of `Meters` and `Grams`), where scalac's is an `int`.
final class Meters(val v: Double) extends AnyVal
final class Feet(val w: Int) extends AnyVal
final class Grams(val g: Long) extends AnyVal
def f(x: Meters & Feet): Int = x match { case y if y.w > 0 => 1 }
def g(x: Feet & Meters): Int = x match { case y if y.w > 0 => 1 }
def k(x: Meters & Feet): Int = x match { case _ => x.w }
def any(x: Meters & Feet): Any = x
def head(x: Meters & Feet): Int = List(x).head.w
def text(x: Meters & Feet): String = x.toString
def three(x: Grams & Meters & Feet): Any = x
def threeLeft(x: Meters & Feet & Grams): Any = x
def opt(x: Meters & Feet): Option[Meters & Feet] = Some(x)
def boxed(e: MatchError): String = e.getMessage.replaceAll("^.*\\(of class ", "(of class ")
def named(a: Any): String = a.getClass.getSimpleName
@main def run(): Unit =
  val x = new Feet(3).asInstanceOf[Meters & Feet]
  println(f(x))
  try println(f(new Feet(-1).asInstanceOf[Meters & Feet])) catch { case e: MatchError => println(boxed(e)) }
  try println(g(new Feet(-2).asInstanceOf[Feet & Meters])) catch { case e: MatchError => println(boxed(e)) }
  println(k(new Feet(5).asInstanceOf[Meters & Feet]))
  println(s"${named(any(x))} ${head(x)} ${text(x).takeWhile(_ != '@')} ${named(three(new Feet(8).asInstanceOf[Grams & Meters & Feet]))} ${named(opt(x).get: Any)} ${named(threeLeft(new Feet(9).asInstanceOf[Meters & Feet & Grams]))}")
