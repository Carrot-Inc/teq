// jars: companionmethod-lib
// An object and a method of one name in a scalac-compiled jar's object, alternatives of the name (dotty's
// module val and method): scalacheck's `Seed`, whose bodies apply the companion of its private
// `case class apply` beside `def apply(s: Long)`, and an object applied through its `apply` beside a
// method, from the jar's bodies and from the program.
import cmlib.*

@main def run(): Unit =
  val s = Seed(5L)
  println(s.value)
  println(s.next.next.value)
  println(Seed.fromLongs(2, 3).next.value)
  println(Lookup.find(4))
  println(Lookup.find(2, 3))
