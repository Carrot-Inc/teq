trait Root[A]:
  def choose[U](u: U): A = null.asInstanceOf[A]
  def choose(n: Int): Int = n
trait Call[A] extends Root[A]:
  def call[U](u: U): A = super.choose[U](u)
class Text extends Root[String]:
  override def choose[U](u: U): String = "text"
class Narrow extends Text with Call[String]

trait Value:
  def value: Any = "root"
trait Read extends Value:
  def read: Any = super.value
class Concrete extends Value:
  override def value: String = "narrow"
class Mixed extends Concrete with Read

@main def run(): Unit =
  println(new Narrow().call(1))
  println(new Mixed().read)
