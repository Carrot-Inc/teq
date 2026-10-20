// Arguments written out of parameter order are lifted into vals before the call only where one of
// them is no safe argument (`Applications`' reordering: `!typedArgs.forall(isSafeArg)`), and then
// each argument that is not pure (`EtaExpansion.liftArgs`, `TreeInfo.isPureExpr`): a stable path, a
// literal, a case class's synthetic `apply` over pure arguments and a default getter's call stay in
// place, an effectful call or a reference through an object with initialisers is lifted in the written
// order, so the effects run as written.
object Marks:
  var log = ""
  def mark[T](s: String, t: T): T = { log += s; t }
  def show(): Unit = { println(log); log = "" }
enum Conn:
  case And, Or
case class Group(items: List[Int], connective: Conn, note: String = Marks.mark("d", "none"))
case class Pair(a: Int, b: Int)
object Pure:
  val p = Pair(1, 2)
object Test:
  def main(args: Array[String]): Unit =
    val g1 = Group(connective = Marks.mark("c", Conn.And), items = Marks.mark("i", List(1)))
    println(g1); Marks.show()
    val g2 = Group(connective = Conn.Or, items = Nil)
    println(g2); Marks.show()
    val g3 = Group(note = Marks.mark("n", "x"), connective = Conn.And, items = Marks.mark("i", List(2)))
    println(g3); Marks.show()
    val g4 = Group(connective = Conn.And, items = List(3), note = "y")
    println(g4); Marks.show()
    val p1 = Pair(b = Marks.mark("b", 2), a = Marks.mark("a", 1))
    println(p1); Marks.show()
    val p2 = Pair(b = Pure.p.a, a = Pure.p.b)
    println(p2); Marks.show()
