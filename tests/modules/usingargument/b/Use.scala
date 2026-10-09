package uab

import uaa.*

@main def run(): Unit =
  println("ab".f1(2)("-"))
  println("ab".f2(2)(using "+"))
  println("ab".f3(2)(using "-")(using '!'))
  println("b".f4(3)(using '?'))
  println(joined(List(1, 2, 3))(using ","))
  println(framed(List(1, 2))(using ";"))
  println(relay(List(4, 5))(using "/"))
  val t = Table(Vector("x", "yz"))
  println(t.row(using " "))
  println(t.padded(3)(using '.'))
  locally {
    given String = "~"
    given Char = '#'
    println("c".f3(2))
    println(inferred(List(6, 7)))
    println(t.row + t.padded(2))
  }
