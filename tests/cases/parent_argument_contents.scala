// What a parent's argument defines is typed outside the class too, however deep: a lambda, a local method, a
// block's class with a parent call, a secondary constructor's call or a deferred given of its own; and the
// parents' arguments are evaluated in the order the parents are written. The code stands in the class's
// constructor: a class of the argument may qualify a member by the class (`private[C]`), and `this.type` in a
// pattern is the class around's.
import scala.compiletime.deferred

class Holder(val v: Int)

trait HasValue:
  given v: Int = deferred
  def get: Int = summon[Int]

class Outer:
  val seed: Int = 10
  class Plain(val n: Int)
  class WithThis extends Plain({ class Local extends Holder(this.seed); new Local().v }) { val seed: Int = 99 }
  class Bare extends Plain({ class Local extends Holder(seed); new Local().v }) { val seed: Int = 99 }
  trait Sized(val m: Int)
  class Scoped extends Plain({ class Local { private[Scoped] def secret: Int = seed }; new Local().secret }) { val seed: Int = 99 }
  class ScopedTrait extends Sized({ class Local { private[ScopedTrait] def secret: Int = seed }; new Local().secret }) { val seed: Int = 99 }
  class Matched(x: Any) extends Plain(x match { case _: this.type => seed; case _ => 0 })
  class MatchedTrait(x: Any) extends Sized(x match { case _: this.type => seed; case _ => 0 })

object Test:
  val log = new StringBuilder
  def note(k: Int): Int = { log.append(k); k }
  def raise(k: Int): Int = k + 1
  val seed: Int = 10
  given around: Int = 10
  trait Sized(val n: Int)
  trait Thunk(val run: () => Int)
  class Base(val a: Int)
  trait First(val b: Int)
  trait Second(val c: Int)
  trait Pair(val x: Int, val y: Int)
  class Lambda(v: Int) extends Thunk(() => raise(v))
  class LocalMethod extends Sized({ def twice(k: Int): Int = raise(k) * 2; twice(4) })
  class Ordered extends Base(note(1)), First(note(2)), Second(note(3)) { log.append("m") }
  class NamedArguments extends Pair(y = note(5), x = note(4))
  class NestedParent extends Sized({ class Local extends Holder(seed); new Local().v }) { val seed: Int = 99 }
  class NestedSecondary extends Sized({ class Local(val v: Int) { def this() = this(seed) }; new Local().v }) { val seed: Int = 99 }
  class NestedGiven extends Sized({ class Local extends HasValue; new Local().get }) { given inner: Int = 99 }
  def main(args: Array[String]): Unit =
    println(s"${new Lambda(4).run()} ${new LocalMethod().n}")
    val o = new Ordered()
    val p = new NamedArguments()
    println(s"${o.a}${o.b}${o.c} ${p.x}${p.y} $log")
    println(s"${new NestedParent().n} ${new NestedSecondary().n} ${new NestedGiven().n}")
    val out = new Outer
    println(s"${new out.WithThis().n} ${new out.Bare().n}")
    println(s"${new out.Scoped().n} ${new out.ScopedTrait().m}")
    println(s"${new out.Matched(out).n} ${new out.Matched("s").n} ${new out.MatchedTrait(out).m} ${new out.MatchedTrait("s").m}")
