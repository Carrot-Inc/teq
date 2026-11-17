// A case's body that begins with a block is continued as any indented block is: by a selection on the line that
// closes it, in a match written in braces, in a block of cases that is a function, after a catch's one case; a
// block in braces on the case's line is continued on that line, and a block inside the body by a line inside it.
// scalac prints the same lines.
def inBraces(x: Int): String = x match {
  case 1 =>
    val s = "ab"
    s
  .toUpperCase
  case _ => "cd"
}

def bracedBody(x: Int): String = x match {
  case 1 => { val s = "ef"; s }.toUpperCase
  case _ => "gh"
}

def insideBody(x: Int): String = x match
  case 1 =>
    val s =
      "ij"
    .toUpperCase
    s
  case _ => "kl"

def caught(x: Int): String =
  try
    if x == 1 then throw new RuntimeException("mn") else "ok"
  catch case e: RuntimeException =>
      val s = e.getMessage
      s
    .toUpperCase

@main def main(): Unit =
  println((inBraces(1), inBraces(2), bracedBody(1), bracedBody(2), insideBody(1), insideBody(2)))
  val function = List(1, 2).map:
    case 1 =>
      val s = "op"
      s
    .toUpperCase
    case _ => "qr"
  println((function, caught(1), caught(2)))
