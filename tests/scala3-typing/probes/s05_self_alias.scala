trait Foo:
  self =>
  def n: Int
  def twice = self.n * 2
class Bar extends Foo { me =>
  def n = 3
  def thrice = me.n * 3
}
@main def run(): Unit = println(Bar().twice + Bar().thrice)
