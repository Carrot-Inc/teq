package gdc

import gda.*
import gdb.*

class Down extends Sorted(using Ordering.Int.reverse)
class SubBox extends IntBox(using 7)

object Here:
  given String = "here"
  class Local extends Named

object Main:
  def main(args: Array[String]): Unit =
    val p = new Site.Plain
    println(Counter.made)
    println(s"${p.show} ${p.name} ${Counter.made}")
    println(Site.Obj.show)
    println(Sorted(using Ordering.Int).sorted(List(3, 1, 2)))
    println(Down().sorted(List(3, 1, 2)))
    println((new Here.Local).show)
    println(WithShown().shown(using 5))
    val b: Boxed[Int] = SubBox()
    println(s"${b.get + 1} ${b.value} ${SubBox().value + 1}")
    println(gda.Access.read(QualifiedImpl(using 31)))
