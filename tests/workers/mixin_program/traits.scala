// A trait of the program that calls `super`: the class mixing it in binds the call, and a
// macro's run on any worker makes the class, whichever worker typed the trait's body.
trait B:
  def f: Int = 1

trait T extends B:
  override def f: Int = super.f + 1

class C extends B with T
