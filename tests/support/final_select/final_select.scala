// `Constructors.intoConstr` switches an identifier naming the class's immutable parameter accessor to the
// constructor's parameter, and a selection `this.s` where the accessor is effectively final (`isEffectivelyFinal`:
// final, private, or a member of a final class): the constructor loads its parameter for those, and reads the accessor
// of one a subclass may override, or of a var (tests/support/final_select/list.sh).
package fs

class A(final val s: Int):
  val saved = this.s
class P(private val s: Int):
  val saved = this.s
  def get = s
final class F(val s: Int):
  val saved = this.s
final case class K(s: Int):
  val saved = this.s
class Q(s: Int):
  val saved = this.s
  def get = s
class O(val s: Int):
  val saved = this.s
  val ident = s
class V(var s: Int):
  val saved = this.s

@main def run(): Unit =
  println(Seq(A(1).saved, P(2).saved, F(3).saved, K(4).saved, Q(5).saved, O(6).saved, O(6).ident, V(7).saved).mkString(" "))
