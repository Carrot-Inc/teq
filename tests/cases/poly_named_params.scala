// Polymorphic function types whose function type names its parameters: `[T] => (x: T) =>
// List[T]`, and results that name them, `x.type` and `c.T` for a `c` of a bounded type
// parameter, with inferred and explicit type arguments.
trait Ctx:
  type T
  def make: T
object IntCtx extends Ctx:
  type T = Int
  def make: Int = 7
@main def run(): Unit =
  val wrap: [T] => (x: T) => List[T] = [T] => (x: T) => List(x)
  println(wrap(1))
  println(wrap[String]("a"))
  val self: [T] => (x: T) => x.type = [T] => (x: T) => x
  val one: 1 = self(1: 1)
  val s: String = self("q")
  println(s"$one $s")
  val dep: [A <: Ctx] => (c: A) => c.T = [A <: Ctx] => (c: A) => c.make
  val n: Int = dep(IntCtx)
  println(n)
