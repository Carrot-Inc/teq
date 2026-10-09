trait T:
  def f(): String
trait T2:
  val t: T
  def u: T
  def g: String = t.f() + "/" + u.f()
object Foo extends T2:
  object t extends T:
    def f(): String = "hello"
  object u extends T:
    def f(): String = "world"
object Test:
  def t2(): T2 = Foo
  def main(args: Array[String]): Unit =
    println(t2().t.f())
    println(t2().u.f())
    println(t2().g)
    println(Foo.t.f())
