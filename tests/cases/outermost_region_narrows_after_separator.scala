  val first = 1

@main def main(): Unit =
  def twice(n: Int) = n * 2
  println(twice(first))
// The file's first line is indented by two spaces: the outermost region is two spaces wide, and the line left of
// it, though a separator ends the statement before it, makes it as wide as itself, so the body after `=` on the
// lines after, at two spaces, is an indented block. (The text comes after the program: a comment line first would
// put the first token on another line.) scalac prints 2.
