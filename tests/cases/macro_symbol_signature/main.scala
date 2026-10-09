package app
import siglib.Sigs

class Box[A](val a: A)

object Api:
  def plain(name: String, count: Int): Boolean = name.length == count
  def poly[A, B](a: A, bs: List[B])(flag: Boolean): Option[A] = if flag then Some(a) else None
  def arr(xs: Array[Int], box: Box[String]): Unit = ()
  def unit(): Unit = ()
  def nothing: Nothing = throw new Exception
  def self: Api.type = this
  def vararg(xs: Int*): Int = xs.sum
  def byName(x: => Long): Long = x
  val field: Double = 1.5

@main def run(): Unit =
  println(Sigs.of[Api.type]("plain"))
  println(Sigs.of[Api.type]("poly"))
  println(Sigs.of[Api.type]("arr"))
  println(Sigs.of[Api.type]("unit"))
  println(Sigs.of[Api.type]("vararg"))
  println(Sigs.of[Api.type]("byName"))
  println(Sigs.of[Api.type]("field"))
  println(Sigs.of[Api.type]("nothing"))
  println(Sigs.of[Api.type]("self"))
