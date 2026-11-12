final class Person(val name: String)
object types:
  opaque type Bounded <: String = String
  def bounded(s: String): Bounded = s
  opaque type Name = String
  def name(s: String): Name = s
  opaque type Id = Int
  def id(i: Int): Id = i
  opaque type Boxed = Person
  def boxed(p: Person): Boxed = p
  opaque type Wrapped[+T, P] <: T = T
  def wrapped[T, P](t: T): Wrapped[T, P] = t
  opaque type Either2 = Int | Person
  def either2(i: Int): Either2 = i

import types.*

@main def run(): Unit =
  val b = bounded("b")
  val n = name("n")
  val i = id(1)
  val x = boxed(Person("x"))
  val w: Wrapped[String, Int] = wrapped("w")
  val wp: Wrapped[Person, Int] = wrapped(Person("wp"))
  val e = either2(2)
  val p = Person("p")
  println(s"1 $b")
  println(s"2 $n")
  println(s"3 $i")
  println(s"4 ${x.toString.length > 0}")
  println(s"5 $w")
  println(s"6 ${wp.toString.length > 0}")
  println(s"7 $e")
  println(s"8 ${p.toString.length > 0}")
  val s9 = s"9 $x"
  val s10 = s"10 $wp"
  val s11 = s"11 $p"
  println(s9.length > 0 && s10.length > 0 && s11.length > 0)
