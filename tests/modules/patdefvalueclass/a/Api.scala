package pvc

// A value class and an opaque type with companions' `apply`s and an extension, upstream.
final class Headers(val headers: List[String]) extends AnyVal {
  def size: Int = headers.length
}
object Headers {
  def apply(hs: String*): Headers = new Headers(hs.toList)
  val empty: Headers = new Headers(Nil)
}

object Bodies {
  opaque type Body = List[Int]
  object Body {
    def apply(xs: Int*): Body = xs.toList
    extension (b: Body) def total: Int = b.sum
  }
}

object Client {
  import Bodies.Body
  def send(h: Headers, b: Body): String = s"${h.size} ${Body.total(b)}"
}
