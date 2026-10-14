package exdb

import exd.*

@main def run(): Unit =
  val s: String = id(new Text)
  println(s)
  val i: Int = id(new Num) + 1
  println(i)
  val l: List[String] = put(new Text)("more")
  println(l)
  val g: Int = Facade.get(new Num)
  println(g)
