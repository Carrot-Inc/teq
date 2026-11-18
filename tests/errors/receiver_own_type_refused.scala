// A receiver's own type that the variable's given or bound refuses. An extension whose context bound the alias
// of a union does not meet (only the widened type's given is there): scalac reports the member as not found at
// the selection; teq the missing context argument at the application's end, as for any extension whose context
// argument is missing. An application's own type is its result's, `String`, which an expected `Cell[Step]`
// refuses.
import scala.language.implicitConversions
trait Label[A]
final class Cell[A](val a: A)
final class Lift[A](private val a: A) extends AnyVal:
  def lifted: Cell[A] = Cell(a)
implicit def toLift[A](a: A): Lift[A] = new Lift(a)
extension [A](a: A) def boxed: Cell[A] = Cell(a)

object Steps:
  type Step = "first" | "second"
  given Label[String] with {}
  extension [A: Label](a: A) def labelled: Int = 1
  val step: Step = "first"
  val result = step.labelled
  def same[A](a: A): A = a
  val viewed: Cell[Step] = same("first").lifted
  val extended: Cell[Step] = same("first").boxed
// expect: 19:29: error: no given instance of type Label["first" | "second"] was found for parameter evidence$1
// expect: 21:28: error: type mismatch: found Cell[String], required Cell["first" | "second"]
// expect: 22:30: error: type mismatch: found Cell[String], required Cell["first" | "second"]
