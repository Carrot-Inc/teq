// jars: quotenest-lib
// A library macro's type pattern binding a type in a contravariant position (`'[Pipe[in, e, out]]` of
// `Pipe[-I, +E, +O]`), `@fromAbove` to scalac, which the 28.4 pickle does not carry: the variable is the argument's type.
import qnest.*

@main def run(): Unit =
  println(Types.inOf[Pipe[Int, Nothing, Boolean]])
  println(Types.inOf[Pipe[Long, Nothing, Char]])
