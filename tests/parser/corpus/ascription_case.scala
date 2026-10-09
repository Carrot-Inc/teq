package ascriptioncase

trait Show[A]:
  def show(a: A): String

extension [A](a: A)(using s: Show[A]) def shown: String = s.show(a)

enum Adt:
  case Queued
  case Progress(done: Int, total: Int)

object Adt:
  given Show[Adt] = a => "adt:" + a.toString

enum Color:
  case Red
  case Custom(r: Int)

object Color:
  given Show[Color] = c => "color:" + c.toString

def describe[A](a: A)(using s: Show[A]): String = "[" + s.show(a) + "]"

@main def main(): Unit =
  println((Adt.Progress(1, 2): Adt).shown)
  val c: Adt = Adt.Progress(3, 4)
  println(c.shown)
  println((Adt.Queued: Adt).shown)
  println((Color.Custom(1): Color).shown)
  println(describe(Color.Custom(2): Color))
  println(describe[Adt](Adt.Progress(5, 6)))
