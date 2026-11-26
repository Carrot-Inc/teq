// A local trait's member that reads the block, overridden by the class mixing it in (a named class, an anonymous
// one): a call through the trait's type runs the trait's own lifted body, as scalac 3.8.4 runs it (dotty's
// `LambdaLift`: the lifted method takes the block's values, and the class's member of the written signature does
// not override it), and a call through the class's or the interface's type runs the override. JavaScript wrote no
// trait body for the lifted member where every class mixing the trait in overrode the written one, and died with
// `TypeError: $1.m$p1 is not a function` (the reach's walk by the source name stopped at the override).
trait I { def m(a: Int, b: Int): Int }
object Lib:
  def named(x: Int): String =
    trait T extends I { def m(a: Int, b: Int = 5): Int = a * 100 + b * 10 + x; def k: Int = m(2) }
    class C extends T { override def m(a: Int, b: Int): Int = a * 100 + b * 10 + 7 }
    val inst: Any = new C
    "T=" + inst.asInstanceOf[T].m(2) + " C=" + inst.asInstanceOf[C].m(2) + " I=" + inst.asInstanceOf[I].m(2, 5) + " self=" + inst.asInstanceOf[T].k
  def anonymous(x: Int): Int =
    trait T { def m(a: Int = x + 3, b: Int): Int = a * 10 + b + x }
    (new T { override def m(a: Int, b: Int): Int = a * 100 + b }).m(b = 1)
@main def run(): Unit =
  println(Lib.named(1))
  println(Lib.anonymous(10))
