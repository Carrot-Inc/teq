class Box(val n: Int):
  def size: Int = n
  def put(x: Int): Int = x
class Ops(val s: String):
  def size: Int = 0
  def put(x: Int): Int = x
object Own:
  extension [U <: Box, T <: U](t: T) def size: Int = 0
object Fwd:
  extension [A <: Box](b: A)
    def ops: Ops = Ops(b.toString)
    export ops.{size, put}
object Enclosing:
  def outer[T <: Box](t0: T): Int =
    extension (t: T) def size: Int = 0
    t0.n
class InClass[T <: Box]:
  extension (t: T) def size: Int = 0
object Ext:
  extension [T <: Box](t: T) def size: Int = 0
object Lead0:
  export Ext.size
object Long:
  extension [A <: Box, B <: A, C <: B, D <: C, E <: D, F <: E, G <: F, H <: G, I <: H, J <: I](t: J) def size: Int = 0
object Main:
  def main(args: Array[String]): Unit = println(Enclosing.outer(Box(3)))
