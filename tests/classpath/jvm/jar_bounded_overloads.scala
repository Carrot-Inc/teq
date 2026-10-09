// jars: scala-library erasure-lib
// std: scala-library
// A call into a scalac-built jar names the descriptor of the erasure of the pickled signature,
// where a type parameter erases to its bound: of tests/support/erasure_lib.scala's overloads
// `f[T <: String](x: T)` and `f(x: Object)`, `f[String]("x")` is the first, as scalac picks it.
// A subclass of the jar's `BoundBox[T <: Named]` overrides `get` and `put` through bridges of
// the jar's descriptors.
import erasure.{Bounded, BoundBox, Cm, Named}

class Person(val name: String) extends Named
class PersonBox extends BoundBox[Person](new Person("p")):
  override def get: Person = new Person("override")
  override def put(x: Person): String = "person " + x.name

@main def main(): Unit =
  println(Bounded.f[String]("x"))
  println(Bounded.f("y"))
  println(Bounded.g[Int](3))
  println(Bounded.arr[String](Array("a", "b")))
  println(Bounded.wrap(new Cm(7)))
  val box: BoundBox[Person] = new PersonBox
  println(box.get.name)
  println(box.put(new Person("q")))
  println(new BoundBox(new Person("r")).get.name)
