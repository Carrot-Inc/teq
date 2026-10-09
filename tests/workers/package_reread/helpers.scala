package shared

import sourcecode.{FullName, Line, Name}

object Utils:
  def fc(body: => String)(using name: FullName, line: Line): String =
    s"[${name.value}:${line.value}] " + body

  def named(body: String)(using name: Name): String =
    withPosition(body)

  private def withPosition(body: String)(using name: Name, line: Line): String =
    s"${name.value}@${line.value} " + body

def describe(tag: String)(using n: Name, f: FullName, l: Line, file: sourcecode.FileName): String =
  s"$tag: ${n.value} | ${f.value} | ${file.value}:${l.value}"


// Kept far down so that its line does not exist in the shorter files.






val inferredLabel = describe("inferredLabel")
