// A val without a written type of the name of an ancestor's private implicit overrides nothing and is typed from
// its right-hand side (Namer.inferredResultType without `inherited`); the class's implicit members, the private
// one not inherited, hold `foo` (scalac 3.8.4 prints foo).
class Foo:
  override def toString = "foo"
trait Ex:
  private implicit val label: String = "x"
  given foo: Foo = Foo()
  def shown: String
object Main:
  def run(): String =
    val e = new Ex:
      val label = summon[Foo].toString
      def shown = label
    e.shown
  def main(args: Array[String]): Unit = println(run())
