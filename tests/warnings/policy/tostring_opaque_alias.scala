object types:
  opaque type Key = Int | String
  def key(i: Int): Key = i
  opaque type Slot = Unit | String
  def slot(s: String): Slot = s
  opaque type Yes = "yes"
  def yes: Yes = "yes"
  val word: String = "w"
  opaque type Word = word.type
  def theWord: Word = word
  opaque type One = 1
  def one: One = 1
  val count: Int = 7
  opaque type Count = count.type
  def theCount: Count = count
  opaque type W[T] = T
  def wrap[T](t: T): W[T] = t

import types.*

@main def run(): Unit =
  val k = key(1)
  val t = slot("t")
  val y = yes
  val w = theWord
  val o = one
  val c = theCount
  val n: W[W[W[String]]] = wrap(wrap(wrap("n")))
  println(s"1 $k")
  println(f"2 $k")
  println(s"3 $t")
  println(s"4 $y")
  println(f"5 $y")
  println(s"6 $w")
  println(s"7 $o")
  println(s"8 $c")
  println(s"9 $n")
