// The superclasses tests/classpath/js/jar_constructor*.scala extend from a jar: their constructors' code reads their
// parameters by an identifier, in a closure, through an inline method and in a local function and its default, and
// through `this` (scalac's `Constructors.splitStats` and `intoConstr`); one passes its parameter to its superclass.
// `Sorted`'s defs call the sorted set's `collect` through its implicit clause (`collect(pf)(ordering)` in the pickle)
// and its `map` and `zip` through the class's given val, beside the vals its constructor initialises, one of them an
// instance whose method calls such a def (tests/classpath/js/jar_sorted_evidence.scala), as cats'
// `NonEmptySetInstances` makes its `Reducible` and `NonEmptySetOps` calls `collect`. `AliasThis`'s constructor casts
// and ascribes `this` to an alias of `this.type` and to `this.type` itself before an inline call, directly and inside
// its inline methods' bodies, and calls one on an inline method's `this` result of the class's type, of `this.type`
// and of a transparent method, and on one whose body is another's result (tests/classpath/js/jar_alias_this.scala);
// `AliasMac.keep`'s quote splices back a source class's inline call.
package ctorlib

import scala.collection.immutable.SortedSet
import scala.quoted.*

class A(val s: Int):
  val ident = s
  val closure = () => s
  val selected = this.s
  inline def get: Int = s
  val viaInline = get
  val viaLocal = { def g = s; g }
  val viaDefault = { def g(k: Int = s) = k; g() }

class Base(val s: Int):
  val b = s
class Mid(override val s: Int) extends Base(s):
  val m = s

class Sorted(val s: SortedSet[Int]):
  implicit val ord: Ordering[Int] = s.ordering
  val size = s.size
  def collected[B](pf: PartialFunction[Int, B])(implicit B: Ordering[B]): SortedSet[B] =
    implicit val ordering: Ordering[B] = B
    s.collect(pf)
  def zipped: SortedSet[(Int, Int)] = s.zip(s.map(_ * 10))

trait Shows:
  def show(s: SortedSet[Int]): String
abstract class SortedInstances:
  val shows: Shows = new Shows:
    def show(s: SortedSet[Int]): String = Sorted(s).collected { case x if x > 1 => x * 10 }.toString
object SortedInstances extends SortedInstances

class AliasThis(val s: Int):
  type Me = this.type
  inline def im: Int = s
  val aliased = this.asInstanceOf[Me].im
  val direct = this.asInstanceOf[this.type].im
  val ascAlias = (this: Me).im
  val ascDirect = (this: this.type).im
  inline def viaMe: Int = this.asInstanceOf[Me].im
  inline def viaThis: Int = this.asInstanceOf[this.type].im
  inline def viaAscMe: Int = (this: Me).im
  inline def viaAscThis: Int = (this: this.type).im
  val nestedMe = viaMe
  val nestedThis = viaThis
  val nestedAscMe = viaAscMe
  val nestedAscThis = viaAscThis
  inline def self: AliasThis = this
  inline def selfT: this.type = this
  transparent inline def selfTr: AliasThis = this
  val resSelf = this.self.im
  val resSelfT = this.selfT.im
  val resSelfTr = this.selfTr.im
  inline def selfMe: Me = this
  inline def viaInlT: AliasThis = selfT
  inline def viaInlMe: this.type = selfMe
  val resInlT = this.viaInlT.im
  val resInlMe = this.viaInlMe.im

object AliasMac:
  inline def keep(inline x: Int): Int = ${ keepImpl('x) }
  def keepImpl(x: Expr[Int])(using Quotes): Expr[Int] = '{ val r = $x; r }
