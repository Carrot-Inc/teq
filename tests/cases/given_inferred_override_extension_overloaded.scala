// given_inferred_override_extension through an overloaded name: the val overrides the implicit alternative of
// `label` beside another and takes its type (scalac 3.8.4 prints A!).
class Box(val v: String)
object Box:
  given ops: Ops = new Ops {}
trait Ops:
  extension (b: Box) def shout: String = b.v.toUpperCase + "!"
trait Ex:
  implicit def label: String
  def label(i: Int): String = "n"
object Main:
  def run(): String =
    val e: Ex = new Ex:
      val label = Box("a").shout
    e.label
  def main(args: Array[String]): Unit = println(run())
