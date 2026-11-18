// `#` ends no statement and begins none: a type projection goes on over the line break after its `#` and over
// the one before it, while `##`, a method's name, ends its line's statement. scalac prints (1,c,true,2).
class A:
  type B = Int
  type C = String

object Test:
  val x: A#
    B = 1
  val y: A
    #C = "c"
  val h = x.##
  val z = 2
  def main(args: Array[String]): Unit =
    println((x, y, h == 1.##, z))
