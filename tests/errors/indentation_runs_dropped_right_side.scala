// expect: 8:6: error: expected an expression, found 'var'
// A line's indentation is its first two runs of one character: under a method's header led by a space, a tab and
// a space, the lines led by a space, a tab and three spaces stand at the header's own width, and the right side
// after `=` opens no indented block there. scalac: "expression expected but var found" after the `=`, then the
// names the block would have defined not found.
object P:
 	 def run(): String =
 	   var s = ""
 	   if false then
 	 s += "x"
 	   s += "y"
 	   s

@main def main(): Unit = println(P.run())
