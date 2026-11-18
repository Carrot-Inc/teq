// expect: 12:4: error: Incompatible combinations of tabs and spaces in indentation prefixes.
// expect: Previous indent : 2 tabs
// expect: Latest indent   : 1 tab, 2 spaces
// expect: Latest indent   : 6 spaces
// A line indented with a tab and two spaces under a block of two tabs, and then one of six spaces: neither
// indentation begins with the block's, and each line is reported with its own indentation, the first also where it
// begins with an operator whose operand stands on the next line. scalac: the same two errors, with these lines.
object A:
	def f: Int =
		val a = 1
		a
	  +
      2
