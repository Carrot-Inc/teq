// The method applying the function literal is an inner class's: the outer instance of the anonymous
// subclass is the class's around both.
trait Adaptor:
  def name: String
  def extra: String = "none"
trait Adaptors:
  def mk(n: Int): Adaptor

class Suite:
  private val tag = "suite"
  class Stub extends Adaptor:
    override def name: String = "stub:" + tag
  class Inner extends Adaptors:
    override def mk(n: Int): Adaptor = ((i: Int) => new Stub:
      override def extra = "inner" + i)(n)
  def test: String = { val x = new Inner().mk(7); x.name + "/" + x.extra }

@main def run(): Unit = println(Suite().test)
