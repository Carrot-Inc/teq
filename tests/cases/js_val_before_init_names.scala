// Fields named constructor, name and length, read before their initialisers run and after.
class A {
  def read: Int = constructor
  val early: Int = read
  val constructor: Int = 7
  val name: Int = 8
  val length: Int = 9
  override def toString: String = "A"
}
object Main { def main(args: Array[String]): Unit = { val a = new A; println(a.early); println(a.constructor); println(a.name); println(a.length); println(a.toString) } }
