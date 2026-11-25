// A Java class's non-final fields assigned with no receiver written: an instance's inherited field from a
// subclass and from its inner class (putfield on the instance), a static one after an import (putstatic), each
// read back.
import Counter.total

class Tally extends Counter:
  def bump(): Unit =
    count = count + 1
    name = name + "!"
  def shown: String = name
  class Inner:
    def poke(): Unit = count = count + 10

object Main:
  def main(args: Array[String]): Unit =
    val t = Tally()
    t.bump()
    t.bump()
    t.Inner().poke()
    println(t.count)
    println(t.shown)
    total = total + 5
    total = total * 3
    println(total)
    Counter.label = Counter.label + "L"
    println(Counter.label)
