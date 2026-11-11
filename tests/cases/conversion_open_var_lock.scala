import scala.language.implicitConversions

// A view's variable left open for the member it serves (`ops[?A](List(0.5))`: `Ops[?A]` holds it nonvariantly)
// is not locked for the applications of the expression the view is typed in: dotty's `adaptNoArgsImplicitMethod`
// reads the lock `typed` took where that expression's typing began (Typer.scala:4021, 4506), so `tvarsInParams`
// instantiates `?A` to the receiver's type before the search, and `list`, not an ambiguity with `seq`, answers. The
// same through a second view, inside an argument, for an extension on the view's result and one after a member
// of it, and for a for-comprehension's inner generator, whose closure body has a `typed` of its own (`foo`, not an
// ambiguity with `bar`).
trait Enc[A]:
  def name: String
trait EncLow:
  implicit def seq[A](implicit a: Enc[A]): Enc[Seq[A]] = new Enc[Seq[A]] { def name = "seq of " + a.name }
  implicit def list[A](implicit a: Enc[A]): Enc[List[A]] = new Enc[List[A]] { def name = "list of " + a.name }
object Enc extends EncLow:
  implicit val double: Enc[Double] = new Enc[Double] { def name = "double" }
  implicit def box[A](implicit a: Enc[A]): Enc[Box[A]] = new Enc[Box[A]] { def name = "box of " + a.name }
  implicit val foo: Enc[Foo] = new Enc[Foo] { def name = "foo" }
  implicit val bar: Enc[Bar] = new Enc[Bar] { def name = "bar" }
class Bar
class Foo extends Bar

final class Box[A](val a: A):
  def encoded(implicit e: Enc[A]): String = e.name
final class Ops[A](val a: A):
  def self: Box[A] = new Box(a)
  def encoded(implicit e: Enc[A]): String = e.name
  def foreach[U](f: Int => U)(implicit e: Enc[A]): Unit = println("each " + e.name)
implicit def ops[A](a: A): Ops[A] = new Ops(a)

final class BoxOps[B](val b: B):
  def viaBox(implicit e: Enc[B]): String = e.name
implicit def boxOps[B](b: Box[B]): BoxOps[Box[B]] = new BoxOps(b)

extension [B](b: Box[B]) def extWith(n: Int)(implicit e: Enc[B]): String = s"$n " + e.name
extension [B](o: Ops[B]) def opsExt(implicit e: Enc[B]): String = e.name
extension [B](o: Ops[B]) def opsExtWith(n: Int)(implicit e: Enc[B]): String = s"$n " + e.name

def f[T](b: Box[T])(implicit e: Enc[T]): String = e.name

@main def run(): Unit =
  println(List(0.5).encoded)
  println(List(0.5).self.viaBox)
  println(f(List(0.5).self))
  val b = List(0.5).self
  println(b.encoded)
  println(List(0.5).self.extWith(2))
  println(List(0.5).opsExt)
  println(List(0.5).opsExtWith(3))
  new Foo().foreach(i => ())
  for (i <- List(1); j <- new Foo()) ()
