// jars: quotenest-lib
// A library macro's nested quote pattern naming the type an enclosing typed pattern bound (`case given Type[t] => ..
// case '[`t`] =>`): a reference to it, `Int`, which `String` does not match.
import qnest.*

@main def run(): Unit =
  println(Backquoted.against[String])
