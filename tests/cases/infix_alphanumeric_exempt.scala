// jars: scala-library cats-kernel cats-core
//> using dep org.typelevel::cats-core:2.13.0
// Alphanumeric methods written infix that `Checking.checkValidInfix` lets pass, so that no warning is printed: a
// method of a library compiled before 3.4 (cats-core 2.13.0's TASTy is 28.3: `concatNel`), one declared `infix`,
// a backquoted one, one followed by a block, scala-library's (`max`, `map`, which the std stands for) and an
// operator whose name ends in `_` and symbols.
import cats.data.NonEmptyList

class C { infix def plus(x: Int): Int = x + 1; def f(x: Int): Int = x * 2; def a_+(x: Int): Int = x + 3 }

@main def run(): Unit =
  val c = new C
  println((NonEmptyList.of(1, 2) concatNel NonEmptyList.of(3)).toList)
  println(c plus 1)
  println(c `f` 2)
  println(c f { 3 })
  println(1 max 2)
  println(List(1) map (_ + 1))
  println(c a_+ 4)
