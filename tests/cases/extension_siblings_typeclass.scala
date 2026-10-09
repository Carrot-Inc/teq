trait Show[A]:
  extension (a: A)
    def show: String
    def showTwice: String = show + show
    def showIn(l: String, r: String): String = l + show + r
    def bracketed: String = showIn("[", "]")

given Show[Int] with
  extension (a: Int)
    def show: String = s"#$a"

object Ops:
  extension [A](xs: List[A])(using s: Show[A])
    def render: String = xs.map(_.show).mkString(",")
    def renderAll: String = "<" + render + ">"
    def anyMatch(p: A => Boolean): Boolean = xs.exists(x => locate(_ == x).isDefined && p(x))
    def locate(p: A => Boolean): Option[A] = xs.find(p)
    def pick[B](f: A => B): List[B] = xs.map(f)
    def pickStrings: List[String] = pick[String](_.show)

  extension (s: String)
    def isBlankish: Boolean = trimmed.isEmpty
    def trimmed: String = s.trim
    def orElse(other: String): String = if isBlankish then other else trimmed

import Ops.*

@main def main(): Unit =
  println(3.showTwice)
  println(3.bracketed)
  println(List(1, 2).renderAll)
  println(List(1, 2).anyMatch(_ > 1))
  println(List(1, 2).pickStrings)
  println("  ".orElse("dflt") + " x ".orElse("dflt"))
