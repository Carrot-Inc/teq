// codeOf prints its argument's tree as the RefinedPrinter shows it (Inlines' `arg.show`): a type reference with its
// prefix, a package's by its full name (`p.C`, `r.s.C`), a Java nested class's with a dot, and none where the prefix
// is omittable (scala, java.lang, Predef's aliases); a written type as its tree (`Tuple2[Int, String]`, a bare `?`
// with its bounds), a function's, a context function's and a named tuple's as their types.
import scala.compiletime.codeOf
package p:
  class C
  class G[T]
  object obj
  type T = Int
  object O:
    def mk: C = new C
    class In
package q:
  class C
package r.s:
  class C
trait A
trait B
class Pair[X, Y]
infix class Or[X, Y]

@main def run(): Unit =
  println(codeOf(new p.C))
  println(codeOf(p.obj))
  println(codeOf(List.empty[p.C]))
  println(codeOf((x: p.C) => x))
  println(codeOf(p.O.mk))
  println(codeOf(new p.G[q.C]))
  println(codeOf(Option.empty[r.s.C]))
  println(codeOf(null.asInstanceOf[p.T]))
  println(codeOf(classOf[p.C]))
  println(codeOf(new p.O.In))
  println(codeOf(classOf[java.util.Locale]))
  println(codeOf(null.asInstanceOf[java.util.Map.Entry[String, Int]]))
  println(codeOf(Option.empty[String]))
  println(codeOf(List.empty[Option[Int]]))
  println(codeOf(List.empty[Int => String]))
  println(codeOf(List.empty[(Int, String)]))
  println(codeOf(List.empty[A & B]))
  println(codeOf(List.empty[A | B]))
  println(codeOf(List.empty[Int Or String]))
  println(codeOf(List.empty[(Int, String) => Boolean]))
  println(codeOf(List.empty[Array[Int]]))
  println(codeOf(List.empty[Pair[Int, String]]))
  println(codeOf(List.empty[Option[?]]))
  println(codeOf(List.empty[Int ?=> String]))
  println(codeOf(List.empty[(x: Int, y: String)]))
