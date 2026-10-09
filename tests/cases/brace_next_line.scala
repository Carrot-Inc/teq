trait P
{
  def p: Int = 1
}
class A(x: Int) extends P
{
  def a = x + p
}
trait T extends P
{
  def t = 2
}
object O extends T
{
  def o = 3
}
class B[X]
{
  def b = 4
}
enum Color
{
  case Red, Green
}
given P with
{
  override def p = 9
}
object Main {
  def foo(f: => Int) = f + 10
  def main(args: Array[String]): Unit = {
    val c = true
    if (c)
    {
      println("if")
    }
    else
    {
      println("else")
    }
    while (!c)
    {
      println("never")
    }
    for (i <- 1 to 2)
    {
      println(i)
    }
    try
    {
      println("try")
    }
    finally
    {
      println("fin")
    }
    val x = foo(1)
    { println("separate block") }
    println(x)
    println(new A(1).a + new B[Int].b + O.o + O.t)
    println(Color.values.toList)
    println(summon[P].p)
  }
}
