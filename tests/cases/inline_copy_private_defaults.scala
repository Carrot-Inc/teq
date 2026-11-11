// A case class's `copy` in an inline body of another object, the parameters it leaves out private fields: each is
// the copy's public getter (`r.copy$default$2`), which takes no inline accessor, and the body is pickled as scalac's.
final case class Rec(level: Int = 0, private val shown: Boolean = false, private val marks: Vector[Int] = Vector.empty):
  def show: String = s"$level $shown $marks"

object Rec:
  def make: Rec = Rec(1, true, Vector(2))

object Edits:
  inline def raised(r: Rec): Rec = r.copy(level = 5)

@main def run(): Unit =
  println(Edits.raised(Rec.make).show)
  println(Edits.raised(Rec()).show)
