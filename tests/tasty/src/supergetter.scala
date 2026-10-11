package fix.supergetter

// A getter selected on `super` inside a trait: `SuperAccessors` gives it an accessor of its own
// beside the member's (`super$SgLayer$$f$default$1`, of the getter's shape: the method's type
// parameters, the clauses before the parameter's, its type `@uncheckedVariance`), which the
// trait's body calls on its `this`; one taking an earlier clause's argument.
object SgMarks:
  def mark(s: String): Int = s.length

class SgBase:
  def f(a: Int = SgMarks.mark("A")): Int = a
  def g(x: Int)(y: Int = x * 2): Int = x + y

trait SgLayer extends SgBase:
  override def f(a: Int = SgMarks.mark("L")): Int = super.f() + a
  override def g(x: Int)(y: Int = x * 3): Int = super.g(x + 1)() + y

class SgMixed extends SgBase with SgLayer
