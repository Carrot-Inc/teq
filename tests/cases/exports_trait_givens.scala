package traitgivens

import Prelude.{*, given}

trait Show[A]:
  def show(a: A): String

class StrShow extends Show[String]:
  def show(a: String): String = s"str:$a"

trait Monoid[A]:
  def zero: A
  extension (a: A) def |+|(b: A): A

// A given in a trait is reached through the object that mixes the trait in.
trait PreludeCore:
  given Show[Int] with
    def show(a: Int): String = s"int:$a"
  given showList[A](using s: Show[A]): Show[List[A]] with
    def show(as: List[A]): String = as.map(s.show).mkString(",")
  given strShow: Show[String] = StrShow()
  given Monoid[Int] with
    def zero: Int = 0
    extension (a: Int) def |+|(b: Int): Int = a + b
  lazy val baseLazy: String = "lazy"
  def viaGiven: String = summon[Show[Int]].show(3)

object Prelude extends PreludeCore

class Local extends PreludeCore:
  def here: String = summon[Show[String]].show("local") + viaGiven

def render[A](a: A)(using s: Show[A]): String = s.show(a)

@main def run(): Unit =
  println(baseLazy)
  println(render(1))
  println(render(List(1, 2)))
  println(render("x"))
  println(Prelude.viaGiven)
  println(Local().here)
  println(1 |+| 2)
  println(summon[Monoid[Int]].zero)
