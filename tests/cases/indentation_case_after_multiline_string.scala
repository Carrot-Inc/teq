// A case that stands on the line a multiline string closes on is as wide as that line, not as the line the string
// began on: the lines after it led by the same tab stand at its width. scalac prints one.
def f: String =
  """text
	""" match { case x
	if true => "one"
	case _ => "other" }
@main def main(): Unit = println(f)
