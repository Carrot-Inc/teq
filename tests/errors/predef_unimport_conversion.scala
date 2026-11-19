// expect: 11:30: error: Found:    scala.Byte⏎Required: java.lang.Byte
// expect: 12:32: error: Found:    1⏎Required: Integer
// expect: 2 errors found
// An explicit import of `Predef` takes its implicit conversions away with its other members, and
// a wildcard import leaves out the conversions it hides.
import Predef.{byte2Byte as _, int2Integer as _, *}

object Test:
  def run(): Unit =
    val c: java.lang.Character = 'c'
    val b: java.lang.Byte = (1: Byte)
    val i: java.lang.Integer = 1
