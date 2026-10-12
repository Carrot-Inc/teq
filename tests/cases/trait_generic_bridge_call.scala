// A trait implementing a generic parent's method is called through the parent (the bridge), and a case class's
// `productIterator` is `Product`'s: the design pass's run witness (the bridge's placement, `Mixin`/`Bridges`, is
// feat/jvm-6's).
trait Base[A] { def f(x: A): A }
trait T extends Base[String] { def f(x: String): String = x }
class C extends T
case class P(x: Int)
@main def run(): Unit = { println((new C: Base[String]).f("ok")); println(P(1).productIterator.mkString) }
