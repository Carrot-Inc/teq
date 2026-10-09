// expect: method f needs `override` modifier
// expect: method g of type String has incompatible type
// expect: class C needs to be abstract, since def h(x: Int): Int in trait T is not defined
// expect: method nothing overrides nothing
// expect: private method p cannot override method p in trait T
trait T:
  def f: Int = 1
  def g: Int
  def h(x: Int): Int
  def p: Int = 22

class C extends T:
  def f: Int = 2
  def g: String = ""
  def h(x: String): Int = 1
  override def nothing: Int = 3
  private def p: Int = 23

@main def run(): Unit = println(C().f)
