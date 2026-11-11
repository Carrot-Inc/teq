import scala.language.implicitConversions

// A view's variable still open where a later member of the chain names it (`List(0.5).self.encoded`: the view's
// `Ops[?A]` gives the parameterless `self: Box[?A]`, whose `encoded(using Enc[?A])` is the next application). dotty
// adapts the parameterless reference at its widened type and leaves the node alone (Typer.scala:5139, 5188-5199), so
// `tvarsInParams` walks from `encoded` through `self` to the view's `ops[?A]` (Inferencing.scala:396-404) and
// instantiates `?A` to the receiver's type before the search: `list`, not an ambiguity with `seq`, answers. The same
// with argument lists, three selections deep, and for an extension whose receiver is the chain, its using clause
// after a further explicit list (`extAny`, whose `B` is the whole `Box[?A]`).
trait Enc[A]:
  def name: String
trait EncLow:
  implicit def seq[A](implicit a: Enc[A]): Enc[Seq[A]] = new Enc[Seq[A]] { def name = "seq of " + a.name }
  implicit def list[A](implicit a: Enc[A]): Enc[List[A]] = new Enc[List[A]] { def name = "list of " + a.name }
object Enc extends EncLow:
  implicit val double: Enc[Double] = new Enc[Double] { def name = "double" }
  implicit def box[A](implicit a: Enc[A]): Enc[Box[A]] = new Enc[Box[A]] { def name = "box of " + a.name }

final class Box[A](val a: A):
  def encoded(implicit e: Enc[A]): String = e.name
  def encodedWith(n: Int)(implicit e: Enc[A]): String = s"$n " + e.name
  def same: Box[A] = this
final class Ops[A](val a: A):
  def self: Box[A] = new Box(a)
  def selfWith(n: Int): Box[A] = new Box(a)
implicit def ops[A](a: A): Ops[A] = new Ops(a)

extension [B](b: B) def extAny(n: Int)(implicit e: Enc[B]): String = s"$n " + e.name

@main def run(): Unit =
  println(List(0.5).self.encoded)
  println(List(0.5).selfWith(1).encodedWith(2))
  println(List(0.5).self.same.encoded)
  println(List(0.5).self.extAny(3))
