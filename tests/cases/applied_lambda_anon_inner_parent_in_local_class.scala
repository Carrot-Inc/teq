// The method applying the function literal is a named local class's.
trait Adaptor:
  def name: String
  def extra: String = "none"
trait Adaptors:
  def mk(n: Int): Adaptor

class Suite:
  private val tag = "suite"
  class Stub extends Adaptor:
    override def name: String = "stub:" + tag
  def local: Adaptors =
    class Local extends Adaptors:
      override def mk(n: Int): Adaptor = ((i: Int) => new Stub:
        override def extra = "local" + i)(n)
    new Local
  def test: String = { val x = local.mk(8); x.name + "/" + x.extra }

@main def run(): Unit = println(Suite().test)
