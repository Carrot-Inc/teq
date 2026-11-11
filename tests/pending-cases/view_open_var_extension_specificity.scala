import scala.language.implicitConversions

// An extension whose only clause after its receiver is a using clause, on a view's chain (`List(0.5).self.extAny`):
// the extension is tried in a fresh typer state that owns none of the view's variables, so `tvarsInParams` leaves
// the view's `?A` open, and the search for `Enc[Box[?A]]` asks `box` for an `Enc[?A]` with `?A >: List[Double]`.
// dotty settles it by its given-result comparison, `list` over `seq`; teq reports the two as ambiguous and the
// extension does not apply (implicit specificity for invariant result types).
trait Enc[A]:
  def name: String
trait EncLow:
  implicit def seq[A](implicit a: Enc[A]): Enc[Seq[A]] = new Enc[Seq[A]] { def name = "seq of " + a.name }
  implicit def list[A](implicit a: Enc[A]): Enc[List[A]] = new Enc[List[A]] { def name = "list of " + a.name }
object Enc extends EncLow:
  implicit val double: Enc[Double] = new Enc[Double] { def name = "double" }
  implicit def box[A](implicit a: Enc[A]): Enc[Box[A]] = new Enc[Box[A]] { def name = "box of " + a.name }

final class Box[A](val a: A)
final class Ops[A](val a: A):
  def self: Box[A] = new Box(a)
implicit def ops[A](a: A): Ops[A] = new Ops(a)

extension [B](b: B) def extAny(implicit e: Enc[B]): String = e.name

@main def run(): Unit =
  println(List(0.5).self.extAny)
