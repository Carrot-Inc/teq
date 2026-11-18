// expect: 10:5: error: Incompatible combinations of tabs and spaces in indentation prefixes.
// A line of four tabs under cases at four spaces: an indentation of tabs and one of spaces are no prefix of each
// other and cannot be compared. scalac: "Incompatible combinations of tabs and spaces in indentation prefixes"
// at the same line.
def f(x: Int): Any =
  x match
    case 1 => "a"
    case _ =>
      "bcd"
				.length
