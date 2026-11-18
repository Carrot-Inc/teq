// teq: --werror
// expect: 11:5: warning: Line is indented too far to the left, or a `}` is missing
// expect: 18:5: warning: Line is indented too far to the left, or a `}` is missing
// A line of a brace block that stands left of the block's first line, under a lambda's arrow and under a
// case's: the braces hold it, a statement of the body, and the line is reported as indented too far to the
// left. scalac: "Line is indented too far to the left, or a `}` is missing", a warning, at both lines.
def a(): String =
  var log = ""
  List(1, 2).foreach { x =>
      log += "a" + x
    log += "b" + x
  }
  log
def b(): String =
  var log = ""
  List(1, 2).foreach { case x =>
      log += "a" + x
    log += "b" + x
  }
  log
