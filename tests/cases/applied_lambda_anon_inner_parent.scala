// A function literal applied where it is written, in a method of an anonymous class: its body makes an
// anonymous subclass of an inner class of the class around, whose outer instance is that class's.
trait Adaptor:
  def name: String
  def extra: String = "none"
trait Adaptors:
  def mk(n: Int): Adaptor

class Suite:
  private val tag = "suite"
  private class Stub extends Adaptor:
    override def name: String = "stub:" + tag
  def adaptors(response: String): Adaptors = new Adaptors:
    override def mk(n: Int): Adaptor = ((i: Int) => new Stub:
      override def extra = response + i)(n)
  def test: String = { val x = adaptors("resp").mk(7); x.name + "/" + x.extra }

@main def run(): Unit = println(Suite().test)
