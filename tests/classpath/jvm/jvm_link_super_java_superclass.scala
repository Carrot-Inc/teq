// jars: scala-library abi-callbacks-lib
// std: scala-library
// Jar traits over java.util.RandomAccess whose `super.toString()`, `super.hashCode()` and `super.equals(o)` run the JDK's
// bodies through an ArrayList subclass that mixes them in: the class's super accessors call its superclass, as scalac's
// do, where an interface's call printed the object's identity or threw AbstractMethodError.
import abi.{Equated, Hashed, Shown}
class S extends java.util.ArrayList[String] with Shown
class H extends java.util.ArrayList[String] with Hashed
class E extends java.util.ArrayList[String] with Equated
@main def run(): Unit =
  val s = S()
  s.add("x")
  println(s.toString)
  val h = H()
  h.add("x")
  println(h.hashCode)
  val e = E()
  e.add("x")
  val other = java.util.ArrayList[String]()
  other.add("x")
  println(e.equals(other))
