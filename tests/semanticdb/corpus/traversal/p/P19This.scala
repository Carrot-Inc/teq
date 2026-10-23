package p19
class Base { def f = 1 }
trait Mix { def f: Int = 2 }
class D extends Base with Mix:
  override def f = super[Base].f + super.f + this.g + D.this.g
  def g = 1
