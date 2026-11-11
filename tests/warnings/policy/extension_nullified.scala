class Box(val n: Int):
  def size: Int = n
  def get(i: Int): Int = n + i
  def put(c: Char): Unit = ()
  def take(using DummyImplicit)(i: Int): Int = i
trait Sized:
  extension (b: Box) def size: Int
object Impl extends Sized:
  extension (b: Box) def size: Int = 3
object Main:
  opaque type Label = String
  extension (b: Box) def size: Int = 0
  extension (b: Box) def get(i: Int): Int = 1
  extension (b: Box) def put(i: Int): Unit = ()
  extension (b: Box) def take(i: Int): Int = 2
  extension (b: Box) def other: Int = 3
  extension (n: Int) def toLong: Long = 0L
  extension (l: Label) def size: Int = 4
  def main(args: Array[String]): Unit = println(Box(1).get(2) + Box(1).other)
