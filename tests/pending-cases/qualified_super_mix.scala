// A super selection qualified by its class and a parent (`C.super[T].f`): scalac 3.8.4 accepts it, teq reports
// "expected an identifier, found 'super'".
class B:
  def f = 1
trait T:
  def f = 2
class C extends B with T:
  override def f = C.super[T].f + super[B].f
@main def run(): Unit = println(new C().f)
