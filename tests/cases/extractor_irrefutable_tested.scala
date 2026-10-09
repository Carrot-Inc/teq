// teq: --werror
// An extractor whose result cannot fail (a `Some` or a product) covers the values that pass
// the test of what its `unapply` takes, where its sub-patterns cover the result: with
// `Name(_)` beside it the match over the sealed `Tree` is exhaustive, as scalac finds it.
sealed trait Tree
case class Apply(fun: Tree, args: List[Tree]) extends Tree
case class Name(s: String) extends Tree

object Arity:
  def unapply(t: Apply): Some[Int] = Some(t.args.length)
object Parts:
  def unapply(t: Apply): (Tree, Int) = (t.fun, t.args.length)
object Whole:
  def unapply(t: Tree): (Int, Int) = (1, 2)

def arity(x: Tree): Int = x match
  case Arity(n) => n
  case Name(_) => -1
def parts(x: Tree): String = x match
  case Parts(f, n) => s"$f/$n"
  case Name(s) => s
def whole(x: Tree): Int = x match
  case Whole(a, b) => a + b
def pair(x: Tree): Int =
  val Whole(a, b) = x
  a * b

@main def run(): Unit =
  val ap = Apply(Name("f"), List(Name("a"), Name("b")))
  println(arity(ap))
  println(arity(Name("x")))
  println(parts(ap))
  println(parts(Name("y")))
  println(whole(Name("z")))
  println(pair(ap))
