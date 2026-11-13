// Super calls in an order that is not the order of the parent's members.
trait Written extends Ground:
  override def c(): Int = super.c() + 10
  override def a(): Int = super.a() + 10
  override def b(): Int = super.b() + 10

trait Argument extends Ground:
  override def n(x: Int, y: Int): Int = super.n(super.b(), super.a())

trait Operands extends Ground:
  def sum: Int = super.a() + super.b()
  def text: String = "x" + super.c() + super.n(2, 1)
