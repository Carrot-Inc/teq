package shapes

class Shape:
  def name: String = getClass.getSimpleName

class Circle(val r: Double) extends Shape
case class Square(side: Int) extends Shape

object Registry:
  class Entry(val id: Int)

class Failure(msg: String) extends Exception(msg)

def describe(x: Any): String = x.getClass.getName

@main def run(): Unit =
  val c = new Circle(1.0)
  println(c.getClass.getName)
  println(c.getClass.getSimpleName)
  println(c.name)
  println(Square(2).name)
  println(new Registry.Entry(1).getClass.getName)
  println(c.getClass)
  println(c.getClass == new Circle(2.0).getClass)
  println(c.getClass == Square(1).getClass)
  println(c.getClass.isInstance(new Circle(3.0)))
  println(c.getClass.isInstance(Square(1)))
  val s: Shape = Square(3)
  println(s.getClass.getSimpleName)
  println(describe(5))
  println(describe("text"))
  println(describe(true))
  println(new Failure("x").getClass.getName)
  println(new Failure("x").getStackTrace.length >= 0)
  println(locally { val a = 1; a + 1 })
  try throw new Failure("boom")
  catch case e: Exception => println(e.getClass.getSimpleName + ": " + e.getMessage)
