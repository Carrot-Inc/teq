trait Foo:
  self =>
  def n: Int
  def twice = self.n * 2
  def show = s"n=$self"
  def same: Boolean = self.eq(this)
  def alsoType(x: self.type): Int = x.n

class Bar extends Foo { me =>
  def n = 3
  def thrice = me.n * 3
  override def toString = "Bar"
  def viaLambda = List(1, 2).map(i => me.n * i)
}

object Obj { o =>
  val v = 10
  def get = o.v
}

@main def run(): Unit =
  println(Bar().twice + Bar().thrice)
  println(Bar().show)
  println(Bar().same)
  println(Bar().viaLambda)
  println(Obj.get)
  val b = Bar()
  println(b.alsoType(b))
