// `new` of a type lambda that eta-expands a class is a `new` of the class (TypeAssigner.typeOfNew collapses it before
// the constructor is selected), with one parameter or two, in their order. scalac prints the lines of the .expected
// file.
class Box[A](val value: A)
class Pair[A, B](val a: A, val b: B)
@main def run(): Unit =
  val box: Box[Int] = new ([A] =>> Box[A])(1)
  println(box.value)
  val pair = new ([X, Y] =>> Pair[X, Y])("a", 2)
  println(pair.a + pair.b)
  println(new ([A] =>> Box[A])("b").value)
