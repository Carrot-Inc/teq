//> using scala 3.8.4
// Adapted from scala3 tests/run/better-fors-map-elim.scala (Apache-2.0, see tests/scala3/README.md); replaced: the results are printed.
enum MyOption[+A]:
  case MySome(x: A)
  case MyNone

  def map[B](f: A => B): MyOption[B] =
    this match
    case MySome(x) => ???  //MySome(f(x))
    case MyNone => ??? //MyNone
  def flatMap[B](f: A => MyOption[B]): MyOption[B] =
    this match
    case MySome(x) => f(x)
    case MyNone => MyNone
object MyOption:
  def apply[A](x: A): MyOption[A] = MySome(x)

@main def Test =

  val a =
    for
      a <- MyOption(1)
      b <- MyOption(())
    yield ()
  println(a)

  val b =
    for
      a <- MyOption(1)
      b <- MyOption(2)
    yield b
  println(b)

  val c =
    for
      a <- MyOption(1)
      (b, c) <- MyOption((2, 3))
    yield (b, c)
  println(c)

  val d =
    for
      a <- MyOption(1)
      (b, (c, d)) <- MyOption((2, (3, 4)))
    yield (b, (c, d))
  println(d)
