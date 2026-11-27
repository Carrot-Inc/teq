// A local class's constructor default, asked from a lifted body: the getter is the local class's companion's, so the
// enclosing class's lifted methods come in one round.
@main def main(): Unit = {
  class C(val a: Int = 4)
  val f = () => new C().a
  val g = () => 5
  println(f() + g())
}
