// codeOf of an extension call prints dotty's tree of it: the method's own type arguments after the extension's
// clauses (dotty's extension signature interleaves them: `conv(x: Int)[T](f: Int => T)`), a right-associative
// operator's own clause where the receiver's stands, and a package's extension through its package.
import scala.compiletime.codeOf
package p:
  extension (x: Int) def pk: Int = x + 7
  extension (x: Int) def pk2(y: Int): Int = x + y

extension (x: Int) def conv[T](f: Int => T): T = f(x)
extension [A](a: A) def pair[B](b: B): (A, B) = (a, b)
extension (n: Int) def *:(s: String): String = s * n
extension (n: Int) def +:[T](s: T): String = s.toString * n
extension [A](a: A) def ++:[B](b: List[B]): List[Any] = a :: b
extension (n: Int)(using u: DummyImplicit) def *::(s: String): String = s * n

@main def run(): Unit =
  println(codeOf(3.conv[String](_.toString)))
  println(codeOf(3.pair[String]("s")))
  println(codeOf(3.pair("s")))
  println(codeOf(3 *: "ab"))
  println(codeOf(3 +: "ab"))
  println(codeOf(3 ++: List("x")))
  println(codeOf(3 *:: "ab"))
  println({ import p.pk; codeOf(3.pk) })
  println({ import p.pk; { val p = 4; codeOf(p.pk) } })
  println(codeOf(p.pk(3)))
  println(codeOf(p.pk2(3)(4)))
