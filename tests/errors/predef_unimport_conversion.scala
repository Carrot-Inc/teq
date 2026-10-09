// expect: 11:30: error: type mismatch: found Byte, required Byte
// expect: 12:32: error: type mismatch: found Int, required Integer
// expect: 2 errors found
// An explicit import of `Predef` takes its implicit conversions away with its other members, and
// a wildcard import leaves out the conversions it hides.
import Predef.{byte2Byte as _, int2Integer as _, *}

object Test:
  def run(): Unit =
    val c: java.lang.Character = 'c'
    val b: java.lang.Byte = (1: Byte)
    val i: java.lang.Integer = 1
