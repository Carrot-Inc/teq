import scala.language.`2.13`; trait A; trait B; object P { def f(x: A with B): A = x; implicit val i: Int = 1 }
