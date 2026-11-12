// A downstream of a/ as tests/modules builds it. An
// inline parameter's getter is an inline method of its own
// (Desugar.scala:429), whose body makeInlineable prepares:
// a private member it reads goes through the class's inline
// accessor (`inline$k`, PrepareInlineable.scala), so an
// expansion outside the class can read it. scalac 3.8.4 is
// to compile b/ over teq's products and print the .expected
// (tests/modules' scalac-b and scalac-run checks), as every
// run is to print it. The whole build's classes are to be
// the split build's (classes, subset, bundle). It moves to
// tests/modules with the change that makes the two builds one.
import lib7.*
@main def run(): Unit =
  val c = C(3)
  import c.given
  println(summon[String])
  println(c.d)
  println(c.g)
