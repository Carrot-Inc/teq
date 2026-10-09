// Evidence in scope converts: a given or using parameter whose type is a `<:<` (an `=:=` too) is a
// view as a `Conversion` is (dotty's `Implicits.viewCandidateKind`, `isConforms`), and a using
// parameter is a candidate as any given of the body is (the contextual implicits hold it):
// `a.length` of an `a: A` with `ev: A <:< String`, `a` returned as a `String`.
def size[A](a: A)(using ev: A <:< String): Int = a.length
def sizeEq[A](a: A)(using ev: A =:= String): Int = a.length
def asString[A](a: A)(using ev: A <:< String): String = a
def upper[A](xs: List[A])(using ev: A <:< String): List[String] = xs.map(x => x.toUpperCase)
class Wrapped(val s: String)
def unwrap(w: Wrapped)(using c: Conversion[Wrapped, String]): Int = w.length

@main def run() =
  println(size("abc"))
  println(sizeEq("abcd"))
  println(asString("x") + "!")
  println(upper(List("a", "b")))
  given Conversion[Wrapped, String] = _.s
  println(unwrap(Wrapped("hello")))
