package lmao.use

import lmao.lib.*

trait Show[A]:
  def show: String

class ShowOf[A](val show: String) extends Show[A]

object Show extends Both[Show]:
  def split[T](box: Box[Show, T]): Show[T] = new ShowOf[T]("split " + box.name)
  def direct[A](s: String): Box[Show, A] = make[A](s)
  def throughInline[A](s: String): Box[Show, A] = inlined[A](s)

object Outside:
  def selected[A](s: String): Box[Show, A] = Show.make[A](s)

@main def run(): Unit =
  println(Show.direct[Int]("made").name)
  println(Show.throughInline[Int]("inlined").name)
  println(Outside.selected[Int]("selected").name)
  println(Show.derived[Int]("sum").show)
