import scala.compiletime.summonInline
trait Label[A]:
  def text: String
object Labels:
  given label[A]: Label[A] with
    def text = "label"
  inline def reader[A]: () => String = new Function0[String]:
    val captured = summonInline[Label[A]].text
    def apply(): String = captured
  def make = reader[Int]
object Test:
  def main(args: Array[String]): Unit = println(Labels.make())
