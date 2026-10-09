package tfb

import tfa.*

class Both extends Fields with More:
  count += 2
  val own: Int = 7

@main def run(): Unit =
  val b = new Both
  println(b.show)
  println(b.between + b.late + b.late)
  b.flag = false
  println(s"${b.more} ${b.flag} ${b.own}")
