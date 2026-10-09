trait T:
  def f: Int = 1
  def g: Int
  def h(x: Int): Int
class C extends T:
  def f: Int = 2
  def g: String = ""
  def h(x: Int): Int = x
  override def nothing: Int = 3
class E extends T:
  override def f: Int = 3
  val g: Int = 4
  def h(x: Int): Int = x
@main def run(): Unit = println(C().f)
