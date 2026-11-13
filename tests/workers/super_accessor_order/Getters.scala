// A default's getter met after its member, at the right of an operator and at its left.
trait GetterAfter extends Ground:
  def out: Int = { super.opt(1); super.b(); super.opt() }

trait GetterRight extends Ground:
  def out: Int = super.opt() + super.b()

trait GetterLeft extends Ground:
  def out: Int = super.b() + super.opt()
