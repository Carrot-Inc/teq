// expect: 30:28: warning: match may not be exhaustive; missing: Name(_)
// expect: 34:8: warning: unreachable case
// expect: 36:28: warning: match may not be exhaustive; missing: Apply(_, _)
// expect: 42:8: warning: unreachable case
// expect: 44:8: warning: unreachable case
// expect: 44:8: error: unreachable case: type C1 and class C2 are unrelated
// absent: 50:8: warning
// absent: 63:8: warning
// absent: 67:8: warning
// teq: --werror
// An extractor that cannot fail covers what passes the test of its input and no more (`Arity`
// over a `Tree` leaves `Name(_)` missing, and a later `_: Apply` is unreachable); one with an
// `Option` result covers nothing; one whose input no value of the scrutinee can be is never
// reached, an error of its type test besides; as scalac 3.8.4 reports them. A nullary constructor
// and an alternative of extractors before a constructor are judged as their kinds, as scalac's.
sealed trait Tree
case class Apply(fun: Tree, args: List[Tree]) extends Tree
case class Name(s: String) extends Tree
object Arity:
  def unapply(t: Apply): Some[Int] = Some(t.args.length)
object Maybe:
  def unapply(t: Apply): Option[Int] = Some(t.args.length)
object Parts:
  def unapply(t: Apply): (Tree, Int) = (t.fun, t.args.length)
class C1
class C2
object TakesC2:
  def unapply(c: C2): Option[Int] = Some(1)

def covers(x: Tree): Int = x match
  case Arity(n) => n
def covered(x: Tree): Int = x match
  case Arity(n) => n
  case _: Apply => 0
  case Name(_) => 1
def option(x: Tree): Int = x match
  case Maybe(n) => n
  case Name(_) => 1
def product(x: Tree): Int = x match
  case Parts(_, n) => n
  case Name(_) => 1
  case _: Apply => 0
def impossible(x: C1): Int = x match
  case TakesC2(n) => n
  case _ => 0
// `Apply(_, _)` below is unreachable at run time, which scalac does not report, its space engine
// keeping a constructor pattern whole against an extractor of another `unapply`; nor does teq.
def constructorAfter(x: Tree): Int = x match
  case Arity(n) => n
  case Apply(_, _) => 0
  case Name(_) => 1
sealed trait Shape
case class Bare() extends Shape
case class Sized(n: Int) extends Shape
object BareEx:
  def unapply(x: Bare): Some[Int] = Some(1)
object SizedEx:
  def unapply(x: Sized): Some[Int] = Some(x.n)
object SizedEx2:
  def unapply(x: Sized): Some[Int] = Some(x.n)
def nullaryAfter(x: Shape): Int = x match
  case BareEx(n) => n
  case Bare() => 2
  case Sized(n) => n
def alternativeBefore(x: Shape): Int = x match
  case SizedEx(_) | SizedEx2(_) => 1
  case Sized(n) => n
  case Bare() => 2
@main def run(): Unit = println(covers(Name("a")) + covered(Name("b")) + option(Name("c")) + product(Name("d")) + impossible(C1()) + constructorAfter(Name("e")))
