// expect: 10:3: error: Incompatible combinations of tabs and spaces in indentation prefixes.
// expect: Previous indent : 2 spaces
// expect: Latest indent   : 1 tab, 1 space
// A line indented with a tab and a space under a region of two spaces: neither indentation begins with the
// other, so the two cannot be compared and the line is no part of any block. scalac: "Incompatible combinations
// of tabs and spaces in indentation prefixes", with the two indentations, at the same line.
@main def main(): Unit =
  var s = ""
  if false then
	 s += "inside"
   s += "after"
  println("[" + s + "]")
