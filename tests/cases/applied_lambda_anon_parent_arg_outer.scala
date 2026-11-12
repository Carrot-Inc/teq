// The anonymous class's parent is no inner class: its argument reads a member of the class around the
// anonymous class whose method applies the function literal.
trait Adaptor:
  def name: String
  def extra: String = "none"
trait Adaptors:
  def mk(n: Int): Adaptor
abstract class Base(val t: String) extends Adaptor:
  def name: String = "base:" + t

class Suite:
  private val tag = "suite"
  def adaptors: Adaptors = new Adaptors:
    override def mk(n: Int): Adaptor = ((i: Int) => new Base(tag):
      override def extra = "n" + i)(n)
  def test: String = { val x = adaptors.mk(7); x.name + "/" + x.extra }

@main def run(): Unit = println(Suite().test)
