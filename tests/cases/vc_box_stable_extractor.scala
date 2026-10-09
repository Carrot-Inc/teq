// A value class's extractor pattern matches its field as the field: dotty extracts x1._1 before testing it
// (matchArgsPlan), so case C(`key`) compares the Int (EqualTest, pattern == scrutinee); a later pattern on
// the whole value still compares the box.
case class C(u: Int) extends AnyVal
case class L(u: Long) extends AnyVal
object Main {
  val k = 1
  val lk = 1L
  val any: Any = 1
  val boxed: java.lang.Integer = java.lang.Integer.valueOf(1)
  val whole: Any = C(2)
  def main(args: Array[String]): Unit = {
    println(C(1) match { case C(`k`) => "yes"; case _ => "no" })
    println(C(1) match { case C(`any`) => "yes"; case _ => "no" })
    println(C(1) match { case C(`boxed`) => "yes"; case _ => "no" })
    println(L(1) match { case L(`lk`) => "yes"; case _ => "no" })
    println((C(1): Any) match { case C(`k`) => "yes"; case _ => "no" })
    println(C(2) match { case C(`k`) => "field"; case `whole` => "whole"; case _ => "no" })
    println(C(3) match { case C(x) if x == k => "guard"; case C(x) => s"bound $x" })
    try C(4) match { case C(`k`) => println("field") } catch { case _: MatchError => println("MatchError") }
  }
}
