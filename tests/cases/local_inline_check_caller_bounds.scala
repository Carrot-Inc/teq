// A block's inline method called before its definition is checked for that call, and the check runs as the
// definition stands in its block: the bounds a case around the CALL gives a type parameter are the caller's and no
// part of it, so `which(a)` with `a: A` is the `Any` alternative whichever case asked first, as scalac types the
// body in the context the definition was made in. A case around the DEFINITION is the definition's own: there `A`
// is `Int` and the `Int` alternative is taken, and a case nested between the definition and the call adds nothing.
sealed trait Box[A]
case class I(value: Int) extends Box[Int]
case class S(value: String) extends Box[String]

object Called:
  def which(x: Any): String = "any"
  def which(x: Int): String = "int " + (x + 1)
  def which(x: String): String = "string " + x.length
  // The call inside a refining case, the definition after the match.
  def first[A](box: Box[A], x: A): String =
    val inside = box match
      case I(_) => helper(x)
      case _ => "other"
    inline def helper(a: A): String = which(a)
    inside + ", " + helper(x)
  // Two sibling cases, each with a function that calls the method and leaves the match.
  def siblings[A](box: Box[A], x: A): String =
    val call: () => String = box match
      case I(_) => () => helper(x)
      case S(_) => () => helper(x)
    inline def helper(a: A): String = which(a)
    call() + ", " + helper(x)
  // The definition and the call inside one refining case.
  def both[A](box: Box[A], x: A): String = box match
    case I(_) =>
      val early = helper(x)
      inline def helper(a: A): String = which(a)
      early + ", " + helper(x)
    case _ => "other"
  // The definition inside a case, the call in a case nested in it that binds another parameter.
  def nested[A, B](box: Box[A], other: Box[B], x: A, y: B): String = box match
    case I(_) =>
      val early = other match
        case I(_) => helper(x, y)
        case _ => "none"
      inline def helper(a: A, b: B): String = which(a) + "/" + which(b)
      early + ", " + helper(x, y)
    case _ => "other"

@main def run(): Unit =
  println(Called.first(I(1), 2))
  println(Called.first(S("a"), "b"))
  println(Called.siblings(I(1), 2))
  println(Called.siblings(S("a"), "b"))
  println(Called.both(I(1), 2))
  println(Called.both(S("a"), "b"))
  println(Called.nested(I(1), I(2), 5, 6))
  println(Called.nested(I(1), S("s"), 5, "t"))
