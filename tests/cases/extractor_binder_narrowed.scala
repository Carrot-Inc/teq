// The binder of an extractor pattern takes the scrutinee's type where it conforms to what
// `unapply` takes, and otherwise the intersection with it, the narrower where one conforms:
// `apply.fun` of a scalafix rule, overloads chosen by the binder's type, a generic
// `unapply` instantiated against the scrutinee, as scalac's `typedUnApply` and `typedBind`.
sealed trait Tree
case class Apply(fun: Tree, args: List[Tree]) extends Tree
case class Name(s: String) extends Tree

object After:
  def unapply(t: Apply): Option[(Tree, List[Tree])] = Some((t.fun, t.args))
object Narrow:
  def unapply(t: Apply): Option[Tree] = Some(t.fun)
object NarrowSeq:
  def unapplySeq(t: Apply): Option[Seq[Tree]] = Some(t.args)
object Same:
  def unapply[T](t: T): Option[T] = Some(t)
object Wide:
  def unapply(t: Any): Option[Int] = Some(1)
object Bounded:
  def unapply[T <: Apply](t: T): Option[T] = Some(t)
object Heads:
  def unapply[T](t: List[T]): Option[T] = t.headOption

def takes(parent: Option[Tree], sel: Tree): Boolean = parent match
  case Some(apply @ After(_, args)) => (apply.fun eq sel) && args.nonEmpty
  case _ => false

def pick(t: Tree): String = "tree"
def pick(t: Apply): String = "apply"

def narrow(x: Tree): String = x match
  case b @ Narrow(_) => s"${pick(b)} ${b.fun}"
  case _ => "none"
def narrowSeq(x: Tree): String = x match
  case b @ NarrowSeq(first, _*) => s"${pick(b)} ${b.args.length} $first"
  case _ => "none"
def same(x: Tree): String = x match
  case b @ Same(_) => pick(b)
  case _ => "none"
def wide(x: Tree): String = x match
  case b @ Wide(_) => pick(b)
  case _ => "none"
def bounded(x: Tree): String = x match
  case b @ Bounded(c) => s"${pick(b)} ${pick(c)} ${c.args.length}"
  case _ => "none"
def heads(x: Any): String = x match
  case b @ Heads(h) => s"${b.length} $h"
  case _ => "none"
def param[T](x: T): String = x match
  case b @ Narrow(_) =>
    val t: T = b
    s"${pick(b)} ${b.fun} $t"
  case _ => "none"

abstract class Holder:
  type U
  def show(x: U): String = x match
    case b @ Narrow(_) =>
      val u: U = b
      s"${pick(b)} ${b.args} $u"
    case _ => "none"
object Trees extends Holder:
  type U = Tree

def nested(x: Tree): String = x match
  case outer @ (inner @ Narrow(_)) => pick(outer) + pick(inner)
  case _ => "none"

class Keyed[K]:
  def unapply[T <: K](t: T): Some[T] = Some(t)
def keyed(x: Any, k: Keyed[String]): Int = x match
  case k(s) => s.length
  case _ => -1

trait A:
  def a: String = "a"
trait B:
  def b: String = "b"
object TakesB:
  def unapply(b: B): Option[Int] = Some(1)
def overlap(x: A): String = x match
  case v @ TakesB(n) => v.a + v.b + n
  case _ => "none"

@main def run(): Unit =
  val n = Name("x")
  val ap = Apply(n, List(n))
  println(takes(Some(ap), n))
  println(takes(Some(n), n))
  println(narrow(ap))
  println(narrow(n))
  println(narrow(null))
  println(narrowSeq(ap))
  println(same(ap))
  println(wide(ap))
  println(bounded(ap))
  println(bounded(n))
  println(heads(List(3, 4)))
  println(heads("no"))
  println(param(ap))
  println(param(n))
  println(Trees.show(ap))
  println(overlap(new A with B {}))
  println(overlap(new A {}))
  println(nested(ap))
  println(keyed("four", Keyed[String]()) + keyed(4, Keyed[String]()))
