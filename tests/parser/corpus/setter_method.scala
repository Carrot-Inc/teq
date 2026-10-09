class C:
  private var _foo = 0
  def foo: Int = _foo
  def foo_=(v: Int): Unit = _foo = v

@main def run(): Unit =
  val c = C()
  c.foo = 11
  println(c.foo)
