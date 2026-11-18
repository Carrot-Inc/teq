	def f(n: Int): String = n match { case x
	if x == 1 => "one"
	case _ => "other" }
@main def main(): Unit = println(f(1))
// A tab before the file's first token is the first line's indentation, one tab wide, and the lines after it led
// by a tab stand at that width: the cases of a match in braces whose first case begins on the first line go on
// over them. (The text comes after the program: a comment line first would put the first token on another
// line.) scalac prints one.
