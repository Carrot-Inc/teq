object P { def f[T: Ordering](x: T): T = x; val y = f[Int](1)(Ordering.Int) }; import scala.language.`3.3`
