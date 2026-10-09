class Node(val label: String, val next: Node)

def length(n: Node): Int = if n == null then 0 else 1 + length(n.next)

def describe(x: AnyRef): String = x match
  case null => "null"
  case s: String => "string " + s
  case n: Node => "node " + n.label
  case other => "other " + other.toString

def firstOrNull(xs: List[String]): String = xs match
  case Nil => null
  case x :: _ => x

def find[T >: Null <: AnyRef](xs: List[T], p: T => Boolean): T =
  xs.find(p).getOrElse(null)

@main def main(): Unit =
  val chain = new Node("a", new Node("b", null))
  println(length(chain))
  println(length(null))
  println(describe(null))
  println(describe("s"))
  println(describe(chain))
  println(describe(List(1)))
  println(firstOrNull(Nil) == null)
  println(firstOrNull(List("x")) != null)
  val s: String = null
  println(s eq null)
  println(s ne null)
  println(find(List("aa", "b"), (t: String) => t.length == 1))
  println(find(List("aa"), (t: String) => t.length == 3) == null)
  println(Option(s))
  println(Option("v"))
  val xs: List[String] = List("a", null, "c")
  println(xs.map(x => if x == null then "-" else x))
  println(xs.count(_ != null))
  val r: String | Null = null
  println(r == null)
  val nn: Null = null
  println(nn == null)
  println(chain.next.next == null)
  println(if xs.head == null then 1 else 2)
  val anyRef: AnyRef = "boxed"
  println(anyRef.toString)
  println(anyRef.isInstanceOf[AnyRef])
  println(("s": Any).isInstanceOf[AnyRef])
  println(null == null)
  println(List(null, "a"))
