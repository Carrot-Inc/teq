// expect: 10:5: error: expected 'case', found '.'
// The width of a line is the count of its leading spaces and tabs: a line of four tabs under cases at four
// spaces stands at the cases' width and ends them. scalac: "Incompatible combinations of tabs and spaces in
// indentation prefixes" at the same line.
def f(x: Int): Any =
  x match
    case 1 => "a"
    case _ =>
      "bcd"
				.length
