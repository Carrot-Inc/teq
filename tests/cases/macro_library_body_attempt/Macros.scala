//> using dep org.typelevel::cats-core:2.13.0
// jars: scala-library cats-kernel cats-core
// A macro whose run initialises `cats.implicits`, whose instances include `Stream`'s, typed on
// demand for the run inside an attempt of the call (`println`'s overloads): the anonymous class
// of `catsStdInstancesForStream` is the library body's, its typing's reports the attempt's, which
// the retraction drops with it, where scalac typed the body.
import scala.quoted.*

object M:
  inline def go: Int = ${ goImpl }
  def goImpl(using Quotes): Expr[Int] =
    val n = cats.Functor[List](using cats.implicits.catsStdInstancesForList).map(List(1, 2))(_ + 1).sum
    Expr(n)
