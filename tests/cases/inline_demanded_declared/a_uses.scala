// The calls come first among the files, as in inline_demanded_this: `twice` expands at the
// call and the expansion asks for the signatures of `both` and `same`, whose types are
// declared. The class's type parameter in a declared type is the class's, not the type
// argument of the receiver the asking call has: read as the argument, `both` would be a
// `List[Int]` and `same` a method over `Int` for every `Box`.
package demandeddeclared

object Uses:
  def count(): Int =
    val b = new Box[Int](3)
    b.twice

  def first(): String =
    val b = new Box[String]("x")
    b.both.head

  def second(): String =
    val b = new Box[String]("y")
    b.same(b.value)

@main def run(): Unit =
  println(Uses.count())
  println(Uses.first())
  println(Uses.second())
