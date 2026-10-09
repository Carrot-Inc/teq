trait Loud:
  def who: String
class Base extends Loud:
  def who: String = "base"
  def twice: String = who + who
trait Named extends Loud:
  override def who: String = "named"
class Mixed extends Base with Named
class Deeper extends Mixed:
  override def who = "deeper+" + super.who
@main def run(): Unit =
  println(Mixed().twice)
  println(Deeper().twice)
