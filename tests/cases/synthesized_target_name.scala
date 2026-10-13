// A declaration `@targetName` renames meets no synthesized member of its source name in the erased
// duplicate check (`checkNoDoubleDeclaration` pairs by the effective name): the abstract case class's
// generic `toString`, erased as `otherString`, stands beside the synthesized `toString()`.
import scala.annotation.targetName

abstract case class C(n: Int):
  @targetName("otherString") def toString[A]: String

class D(n: Int) extends C(n):
  @targetName("otherString") def toString[A]: String = s"other $n"

@main def run(): Unit =
  val d = D(2)
  println(d)
  println(d.toString[Int])
  val c: C = d
  println(c.toString[Int])
  println(d.productPrefix)
  println(d == D(2))
