// A line's indentation is its first two runs of one character: a line led by a space, a tab and two more spaces
// is as wide as one led by a space and a tab, and one led by a space, two tabs and two spaces as one led by a space
// and two tabs. In a guard, an indented body, a right side, a template's body, a line that begins with an operator
// or a selection, a case's body and a condition's branch, the line with the further spaces goes on as a line of
// its region's width. scalac prints the same line.
object Runs:
 	def guard(n: Int): String = n match {
 	case x if
 	  x == 1 => "one"
 	case _ => "other" }
 	def body(n: Int): Int =
 		val a = n + 1
 		  a * 2
 	def right(n: Int): Int =
 	  n * 3
 	val first = 1
 	  val second = first + 1
 	val sum = 1
 	  + 2
 	val head = List(1, 2).map(_ + 1)
 	  .head
 	def cases(n: Int): String = n match
 		case 1 =>
 			val s = "one"
 			  s
 		case _ => "other"
 	def branch(n: Int): Int =
 		if n > 0 then
 			val one = 1
 			  one
 		else 2
@main def main(): Unit =
  println((Runs.guard(1), Runs.body(1), Runs.right(1), Runs.second, Runs.sum, Runs.head, Runs.cases(1), Runs.branch(1)))
