// `isEffectivelyFinal`'s `Private` is dotty's flag, which a qualified private member (`private[p]`, its
// `privateWithin`) does not have: `this.s` of a `private[p]` accessor a subclass overrides stays a virtual read in the
// constructor's code (B's 2, the JVM's accessor call), where a private one's reads the parameter
// (`Constructors.intoConstr`, D's A got 3); a `protected[p]` one is overridable too. A self tail call of a `private[p]`
// method a subclass overrides is a virtual call, no loop (TailRec's `isEffectivelyFinal`): H's override answers (100).
package p

class A(private[p] val s: Int):
  val saved = this.s
class B(override private[p] val s: Int) extends A(s + 1)
class C(private val s: Int):
  val saved = this.s
class D(val s: Int) extends C(s + 1)
class E(protected[p] val s: Int):
  val saved = this.s
class F(override protected[p] val s: Int) extends E(s + 1)
class G:
  private[p] def f(n: Int): Int = if n <= 0 then 0 else f(n - 1)
class H extends G:
  override private[p] def f(n: Int): Int = if n == 2 then 100 else super.f(n)

@main def run(): Unit =
  println(s"${B(2).saved} ${D(2).saved} ${F(2).saved} ${H().f(3)}")
