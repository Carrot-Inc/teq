// Two expansions of a body that makes a named local class: a class of its own at each site.
trait Base { def x: Int }
inline def make(i: Int): Base = { class Foo extends Base { def x = i }; new Foo }
@main def run(): Unit = println(make(1).x + make(2).x)
