/*
rules = ["file:../../support/MarkStrings.scala"]
*/
// A CRLF file whose multiline string holds an LF alone: text added beside the string is written with the file's
// ending, the string's own bytes as they are (its value stays "a\nb").
object Mixed {
  val s = """a
b"""
}
