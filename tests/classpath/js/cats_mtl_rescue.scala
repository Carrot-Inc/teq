// jars: scala-library cats-kernel-sjs cats-core-sjs cats-mtl
// targets: js interp
// cats-mtl's `rescue` from its jar: the inline body's pattern names the private `Submarine` through
// dotty's receiver-passing accessor, `inline$Submarine$i1(Handle)`, which stands for `Handle.Submarine`
// (`PrepareInlineable.MakeInlineablePassing`).
//> using dep org.typelevel::cats-core:2.13.0
//> using dep org.typelevel::cats-mtl:1.6.0
import cats.mtl.{Raise, Handle}
import cats.mtl.Handle.*

type Result[A] = Either[Throwable, A]

def parse(s: String)(using r: Raise[Result, String]): Result[Int] =
  s.toIntOption match
    case Some(n) => Right(n)
    case None => r.raise(s"bad $s")

def checked(s: String): Result[Int] =
  allow[String]:
    parse(s)
  .rescue: e =>
    Right(-e.length)

@main def run(): Unit =
  println(checked("12"))
  println(checked("x"))
  println(checked("abc"))
