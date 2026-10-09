package cna

class Box(val n: Int)

object Ops:
  def pick(x: Int): String = "int"
  def pick(x: String): String = "string"
  def pick(x: Box): String = "box"
  extension (b: Box) def grow: Box = new Box(b.n + 1)

trait Show[A]:
  def show(a: A): String

object Show:
  given Show[Int] with
    def show(a: Int): String = "i" + a
  given Show[String] with
    def show(a: String): String = "s" + a

trait Meters
object Meters:
  given Conversion[Int, Box] = new Box(_)
