// An indentation of a tab and one of a tab and a space are comparable: the second begins with the first, so the
// lines indented with them are a region under their header as lines indented with spaces are. scalac prints 1.
@main def main(): Unit =
	val x =
	 1
	println(x)
