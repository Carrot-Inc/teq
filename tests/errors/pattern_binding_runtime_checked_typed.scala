// teq: --werror
// `.runtimeChecked` exempts a pattern definition through the type it gives its scrutinee (dotty's
// `RuntimeCheckedAnnot`, read on the typed scrutinee by `Checking.checkIrrefutable`), not through its
// spelling: another class's method of that name, a value stored from it (its inferred type drops the
// annotation), a tuple's element and an ascription of it (the type written) are checked as any right side.
// In `g`, the annotation travels as dotty's typing carries an annotated type: an `if`'s or a match's
// branches on two receivers join to no annotated type, a val declared annotated is read as its
// declared type's singleton, a method declaring it is no `Predef.runtimeChecked` (scalac's message
// names its type `Option[Int] @RuntimeChecked`, the reference's widened), nor is a user's extension of
// that name. A parameter declared annotated is read as its reference, the `TermRef`, which carries none.
final class Fake:
  def runtimeChecked: Option[Int] = Some(7)
def f(): Unit =
  val Some(a) = (new Fake).runtimeChecked
  val checked = Option(7).runtimeChecked
  val Some(b) = checked
  val (Some(c), d) = (Option(7).runtimeChecked, 8)
  val Some(e) = { println("once"); Option(9).runtimeChecked }
  val Some(f) = (Option(10).runtimeChecked: Option[Int])
  ()
class Wrapped
object WrappedOps:
  extension (w: Wrapped) def runtimeChecked: Option[Int] = Some(7)
def annotatedResult: Option[Int] @scala.annotation.internal.RuntimeChecked = Option(7)
def g(args: Array[String], opt: Option[Int]): Unit =
  val Some(a) = (if args.isEmpty then Option(7).runtimeChecked else Option(8).runtimeChecked)
  val Some(b) = (args.length match { case 0 => Option(7).runtimeChecked; case _ => Option(8).runtimeChecked })
  val Some(c) = { val x: Option[Int] @scala.annotation.internal.RuntimeChecked = Option(7); x }
  val Some(d) = { val x: Option[Int] @scala.annotation.internal.RuntimeChecked = Option(7); val y = x; y }
  val Some(e) = { val x: opt.type @scala.annotation.internal.RuntimeChecked = opt; x }
  val Some(f) = annotatedResult
  val Some(h) = { import WrappedOps.runtimeChecked; (new Wrapped).runtimeChecked }
  ()
def param(x: Option[Int] @scala.annotation.internal.RuntimeChecked): Unit =
  val Some(n) = x
// A tuple's component typed by inference from an annotated argument carries none: the type argument drops it.
def components(): Unit =
  val (Some(a), _) = ((Option(10): Option[Int] @scala.annotation.internal.RuntimeChecked), 0)
// expect: 14:17: warning: pattern's type Some[Int] is more specialized than the right hand side expression's type Option[Int]
// expect: 16:17: warning: pattern's type Some[Int] is more specialized than the right hand side expression's type Option[Int]
// expect: 17:22: warning: pattern's type Some[Int] is more specialized than the right hand side expression's type Option[Int]
// absent: pattern_binding_runtime_checked_typed.scala:18:
// expect: 19:18: warning: pattern's type Some[Int] is more specialized than the right hand side expression's type Option[Int]
// expect: 26:18: warning: pattern's type Some[Int] is more specialized than the right hand side expression's type Option[Int]
// expect: 27:18: warning: pattern's type Some[Int] is more specialized than the right hand side expression's type Option[Int]
// expect: 28:17: warning: pattern's type Some[Int] is more specialized than the right hand side expression's type Option[Int]
// expect: 29:17: warning: pattern's type Some[Int] is more specialized than the right hand side expression's type Option[Int]
// expect: 30:17: warning: pattern's type Some[Int] is more specialized than the right hand side expression's type Option[Int]
// expect: 31:17: warning: pattern's type Some[Int] is more specialized than the right hand side expression's type Option[Int]
// expect: 32:17: warning: pattern's type Some[Int] is more specialized than the right hand side expression's type Option[Int]
// expect: 35:17: warning: pattern's type Some[Int] is more specialized than the right hand side expression's type Option[Int]
// expect: 38:22: warning: pattern's type Some[Int] is more specialized than the right hand side expression's type Option[Int]
