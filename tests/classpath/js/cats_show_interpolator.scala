// jars: scala-library cats-kernel-sjs cats-core-sjs
// cats' `show"..."` interpolator: `ShowInterpolator` of `cats.syntax.show`, a `StringContext`
// conversion whose `show` takes `Show.Shown` arguments through the `mat` conversion.
//> using dep org.typelevel::cats-core:2.13.0
import cats.*
import cats.syntax.all.*

case class Slot(index: Int, label: String)
object Slot:
  given Show[Slot] = Show.show(s => s"Slot(${s.index}, ${s.label})")

object Main:
  def p(xs: Any*): Unit = println(xs.mkString(" "))
  def main(args: Array[String]): Unit =
    val a = Slot(2, "b")
    println(show"slot ${a.index} named ${a.label} is $a")
    println(show"list ${List(1, 2)} option ${Option(a)} none ${Option.empty[Slot]} map ${Map("k" -> a)}")
    p(show"plain", show"$a", show"${1.5} ${true} ${'c'} ${2L} ${BigDecimal("1.25")}")
    println(show"${(1, "t")} ${List(a, a)} ${Vector(1)} ${(Right(1): Either[String, Int])} ${Set(3)}")
    val n = 42
    println(show"n=$n, n+1=${n + 1}, nested=${List(Option(n))}")
    p(cats.Show.Shown.mat(a).toString, Show[Slot].show(a), Show.catsShowForString.show("x"))
