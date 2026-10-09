// Overloads that differ only in their arrays' elements, declared in a trait, implemented in a
// class, one of them overridden again below it, and called through each type.
trait Sizes:
  def size(x: Array[Int]): String
  def size(x: Array[String]): String
  def cells(x: Array[Array[Int]]): String = "int grid " + x.map(_.sum).sum
  def cells(x: Array[Array[String]]): String = "string grid " + x.map(_.mkString).mkString

class Sized extends Sizes:
  def size(x: Array[Int]): String = "ints " + x.sum
  def size(x: Array[String]): String = "strings " + x.mkString

class Loud extends Sized:
  override def size(x: Array[String]): String = "STRINGS " + x.mkString.toUpperCase

object Plain:
  def size(x: Array[Int]): String = "plain ints " + x.length
  def size(x: Array[String]): String = "plain strings " + x.length

@main def run(): Unit =
  val all: List[Sizes] = List(new Sized, new Loud)
  for s <- all do
    println(s.size(Array(1, 2)))
    println(s.size(Array("a", "b")))
    println(s.cells(Array(Array(1), Array(2))))
    println(s.cells(Array(Array("x"), Array("y"))))
  val loud = new Loud
  println(loud.size(Array("q")))
  println(Plain.size(Array(1, 2, 3)) + " / " + Plain.size(Array("z")))
