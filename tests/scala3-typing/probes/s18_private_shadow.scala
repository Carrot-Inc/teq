trait A:
  def p: Int
  def getP = p
trait B extends A:
  def p: Int = 22
class C extends B:
  private def p: Int = 23
@main def run(): Unit = println(C().getP)
