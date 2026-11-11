// A val without a written type that overrides an implicit one takes the inherited type, and its right-hand side is
// typed after its completion, not in it (dotty's Namer.inferredResultType: `inherited` before `lhsType`, a def), so
// the class's implicit members read the val complete and the extension the companion's given provides is found
// (scalac 3.8.4 prints A!).
class Box(val v: String)
object Box:
  given ops: Ops = new Ops {}
trait Ops:
  extension (b: Box) def shout: String = b.v.toUpperCase + "!"
trait Ex:
  implicit val label: String
object Main:
  def run(): String =
    val e: Ex = new Ex:
      val label = Box("a").shout
    e.label
  def main(args: Array[String]): Unit = println(run())
