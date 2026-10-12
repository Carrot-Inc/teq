package ob
import oa.*
class Sub extends Base:
  override def alias(x: Types.C): Types.C = (x._1 + 1, x._2)
  override def union(x: Types.U): Types.U = new V(x.x + 1)
  override def plain(x: (Int, String)): (Int, String) = (x._1 + 2, x._2)
class Impl extends Tr:
  def t(x: Types.C): Types.C = x
@main def run(): Unit =
  val b: Base = new Sub
  println(b.alias((1, "a")))
  println(b.union(new V(2)).x)
  println(b.plain((3, "c")))
  println((new Impl: Tr).t((4, "d")))
