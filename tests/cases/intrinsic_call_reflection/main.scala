class Parent:
  override def toString: String = "parent"
class Child extends Parent:
  def shown: String = inspect(super.toString)
@main def run(): Unit =
  val s = "abc"
  println(inspect(s.isEmpty))
  println(inspect(s.toUpperCase))
  println(new Child().shown)
  println(inspect(s.length))
  println(inspect(s.charAt(1)))
  println(inspect(s.substring(1)))
  println(inspect(s.substring(1, 2)))
