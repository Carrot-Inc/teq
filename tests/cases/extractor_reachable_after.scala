// teq: --werror
// A constructor pattern after an extractor that cannot fail, over the extractor's input, and an
// extractor of another `unapply` after one, are no unreachable cases to scalac 3.8.4 (its space
// engine keeps a constructor pattern or an extractor whole against an extractor of another
// `unapply`), so they build under --werror, a nullary constructor (`Bare()`) and an alternative of
// extractors before a constructor included; the binders of type patterns over a union and of a
// generic extractor over an `Any` run as scalac's.
sealed trait Tree
case class Apply(fun: Tree, args: List[Tree]) extends Tree
case class Name(s: String) extends Tree
object Arity:
  def unapply(t: Apply): Some[Int] = Some(t.args.length)
object Arity2:
  def unapply(t: Apply): Some[Int] = Some(t.args.length + 10)
object Parts:
  def unapply(t: Tree): (Int, Int) = (1, 2)
object Always:
  def unapply(t: Tree): Some[Int] = Some(7)

def constructorAfter(x: Tree): Int = x match
  case Arity(n) => n
  case Apply(_, _) => -1
  case Name(_) => 100
def otherExtractor(x: Tree): Int = x match
  case Arity(n) => n
  case Arity2(m) => m
  case Name(_) => 100
def constructorLater(x: Tree): Int = x match
  case Arity(n) => n
  case Name(_) => 100
  case Apply(_, _) => -1
def extractorAfterConstructor(x: Tree): Int = x match
  case Apply(_, Nil) => 0
  case Arity(n) => n
  case Name(_) => 100
def afterProduct(x: Tree): Int = x match
  case Parts(m, n) => m + n
  case Name(_) => 100
def afterSome(x: Tree): Int = x match
  case Always(n) => n
  case Name(_) => 100
  case Apply(_, _) => -1
def bound(x: Tree): String = x match
  case a @ Arity(n) => s"$n of ${a.fun}"
  case b @ Apply(_, _) => "never"
  case Name(s) => s

sealed trait Shape
case class Bare() extends Shape
case class Sized(n: Int) extends Shape
object BareEx:
  def unapply(x: Bare): Some[Int] = Some(1)
object SizedEx:
  def unapply(x: Sized): Some[Int] = Some(x.n)
object SizedEx2:
  def unapply(x: Sized): Some[Int] = Some(x.n + 1)
def nullaryAfter(x: Shape): Int = x match
  case BareEx(n) => n
  case Bare() => 2
  case Sized(n) => n
def alternativeBefore(x: Shape): Int = x match
  case SizedEx(_) | SizedEx2(_) => 1
  case Sized(n) => n
  case Bare() => 2

trait A { def a: Int = 1 }
trait B { def b: Int = 2 }
class C
class AB extends A with B
class CB extends C with B
def unionBinder(x: A | C): Int = x match
  case b: B => b.b
  case _ => 0
class Box[T](val v: T)
object Ex:
  def unapply[T](b: Box[T]): Some[T] = Some(b.v)
def invariant(x: Any): String = x match
  case b @ Ex(v) =>
    val y: Box[?] = b
    val w: Any = v
    s"${y.v} $w"
  case _ => "none"

@main def run(): Unit =
  val app = Apply(Name("f"), List(Name("a"), Name("b")))
  val none = Apply(Name("g"), Nil)
  println(List(constructorAfter(app), otherExtractor(app), constructorLater(none), extractorAfterConstructor(none), extractorAfterConstructor(app)))
  println(List(afterProduct(Name("x")), afterSome(app), constructorAfter(Name("x"))))
  println(List(bound(app), bound(Name("n"))))
  println(List(unionBinder(new AB), unionBinder(new CB), unionBinder(new C)))
  println(List(invariant(Box(3)), invariant(Box("s")), invariant(5)))
  println(List(nullaryAfter(Bare()), nullaryAfter(Sized(4)), alternativeBefore(Sized(5)), alternativeBefore(Bare())))
