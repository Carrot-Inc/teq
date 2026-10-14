// The design pass's probe of the lowered interpreter's obligations (out/codex-probes/lowered-interpreter-design): the order of a case class creation (companion, argument, parent, trait, child) and of an extractor before its guard.
class Parent(x: Int):
  println("parent " + x)
trait T:
  val t: Int = { println("trait"); 2 }
case class Child(x: Int) extends Parent(x) with T:
  val c: Int = { println("child"); 3 }
object Child:
  println("companion")
object Extract:
  def unapply(n: Int): Some[Int] = { println("extract " + n); Some(n) }
@main def run(): Unit =
  println(Child({ println("argument"); 1 }).c)
  println(1 match
    case Extract(n) if { println("guard " + n); false } => 0
    case _ => 9)
