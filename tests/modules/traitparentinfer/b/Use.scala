package tpb

import tpa.*

@main def run(): Unit =
  val i: Int = P().first
  val s: String = O.first
  val q: List[Char] = Q('q').twice
  val r: String = R().first
  val two = Two()
  val l: Int => Int = L.first
  println(i + 1)
  println(s + "!")
  println(q.mkString)
  println(r + R().n)
  println(s"${two.first * 2} ${!two.second}")
  println(l(41))
  println(s"${D().a}${D().b} ${E().x}${E().y}")
