// A value narrowed to `Option[?]` from a union with `String`, `Int` and `Long` is matched by `Some` and `None`
// exhaustively: nothing is both an `Option` and a `String`, `Option` being a class as in scala-library. With the
// std's `Option` a trait, teq warned that the match would fail on `Option[Any] & String`.
final class Tag(val name: String)
final class Empty

type Leaf = Tag | Empty | String | Int | Long
type Node = Leaf | Option[Leaf]

def write(node: Node): String = node match
  case t: Tag => "tag " + t.name
  case _: Empty => "empty"
  case s: String => "string " + s
  case i: Int => "int " + i
  case l: Long => "long " + l
  case o: Option[?] =>
    o match
      case Some(leaf) => "some " + write(leaf.asInstanceOf[Node])
      case None => "none"

@main def run(): Unit =
  println(write(new Tag("a")))
  println(write("b"))
  println(write(Some(3)))
  println(write(None))
