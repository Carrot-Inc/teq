// An enum case whose parameter's type names its enum's type parameter only inside a type annotation's argument
// (`Int @ann(Nil: List[T])`): `typeParamIsReferenced`'s search reads the annotation's tree, so the case takes the
// enum's `T` and `E.C[String]` is its type (scalac 3.8.4 prints 1). teq's type tree keeps an annotation's class and
// not its arguments, so the case takes none and `E.C[String]` is refused: queued with the parser's annotation trees.
class ann(x: Any) extends scala.annotation.StaticAnnotation
enum E[T]:
  case C(x: Int @ann(Nil: List[T])) extends E[Int]
@main def run(): Unit =
  val c: E.C[String] = E.C(1)
  println(c.x)
