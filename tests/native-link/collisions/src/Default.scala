package collide.default
trait Answers {
  def Aa(): Int = 0
  def BB(): Int = 1
  def AaAa(): Int = 2
  def AaBB(): Int = 3
  def BBAa(): Int = 4
  def BBBB(): Int = 5
}
class Plain extends Answers
class Own extends Answers {
  override def AaAa(): Int = 30
  override def BBBB(): Int = 31
  override def BBAa(): Int = 32
}
object Main {
  def main(args: Array[String]): Unit = {
    val a: Answers = if (args.length == 0) new Plain else new Own
    println(a.AaAa())
    println(a.BBBB())
    println(a.AaBB())
    later(new Own)
  }
  def later(o: Own): Unit = {
    println(o.Aa())
    println(o.BB())
    println(o.BBAa())
  }
}
