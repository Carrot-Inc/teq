object Effects:
  final class Task[+A](val value: A)
  type Callback = Task[Unit]
  type Pair[A] = (A, A)
  type Handler[E] = E => Callback

  def run(cb: Callback): String = "ran " + cb.value.toString
  def swap[A](p: Pair[A]): Pair[A] = (p._2, p._1)

object Prelude:
  export Effects.{Callback, Handler, Pair, Task, run, swap}
  type Name = String

import Prelude.*

case class Button(label: Name, onClick: Handler[Int])

def describe(p: Effects.Pair[Int]): String = s"${p._1} then ${p._2}"

@main def main(): Unit =
  val cb: Callback = Task(())
  println(run(cb))
  println(swap((1, 2)))
  println(describe(swap((3, 4))))
  val b = Button("ok", n => Task(println(s"clicked $n")))
  b.onClick(7)
  val who: Prelude.Name = b.label
  println(who)
