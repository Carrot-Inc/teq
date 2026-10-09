// Named imports from a stable val of an object or a package: a val, a parameterless def, an
// overloaded method and a renamed member, each use selecting the member on the val again.
class Counter(start: Int):
  private var n = start
  val label = s"counter from $start"
  def next: Int =
    n += 1
    n
  def add(x: Int): Int = n + x
  def add(x: String): String = s"$x$n"

object Registry:
  val main: Counter = new Counter(10)

val shared: Counter = new Counter(100)

import Registry.main.{label, next, add}
import shared.{next as bump, label as sharedLabel}

@main def run(): Unit =
  println(label)
  println(next)
  println(next)
  println(add(5))
  println(add("n="))
  println(sharedLabel)
  println(bump)
  println(Registry.main.next)
