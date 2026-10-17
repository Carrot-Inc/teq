// A tuple pattern definition over a match whose results are tuple literals (`SimpleTuple`) whose
// components are a value class and an opaque type: the holder's elements are read as the typer reads
// any `$1._1` (Desugar.scala 1650-1657), so a value class's box is unboxed where the component's val
// holds the underlying `List` on the JVM, and the opaque type's value passes as itself.
package patternbindingtuplevalueclass

object Api:
  final class Headers(val headers: List[String]) extends AnyVal:
    def size: Int = headers.length
  object Headers:
    def apply(hs: String*): Headers = new Headers(hs.toList)
    val empty: Headers = new Headers(Nil)

  opaque type Body = List[Int]
  object Body:
    def apply(xs: Int*): Body = xs.toList
    extension (b: Body) def total: Int = b.sum

  def send(h: Headers, b: Body): String = s"${h.size} ${Body.total(b)}"

import Api.*

object Main:
  def request(payload: Option[Int]): String =
    val (finalHeaders, entityBody) = payload match
      case Some(p) =>
        val h = Headers("a", "b")
        (h, Body(p, p))
      case None => (Headers.empty, Body())
    send(finalHeaders, entityBody)

  def main(args: Array[String]): Unit =
    println(request(Some(3)))
    println(request(None))
