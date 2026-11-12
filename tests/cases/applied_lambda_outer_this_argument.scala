// The function literal's argument is the instance of the class around the anonymous class whose method
// applies it, and the body's anonymous class reads the parameter.
trait Adaptor:
  def name: String
  def extra: String = "none"
trait Adaptors:
  def mk(n: Int): Adaptor

class Suite:
  val tag = "suite"
  def adaptors: Adaptors = new Adaptors:
    override def mk(n: Int): Adaptor = ((s: Suite) => new Adaptor:
      def name: String = "anon:" + s.tag
      override def extra = "n" + n)(Suite.this)
  def test: String = { val x = adaptors.mk(7); x.name + "/" + x.extra }

@main def run(): Unit = println(Suite().test)
