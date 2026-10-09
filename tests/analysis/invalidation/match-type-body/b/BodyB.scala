package mtb

import mta.*

// The match type reduced in a body alone, as tests/modules/matchtype's use: the signature
// names neither `Elem` nor its file.
object BodyB:
  def show(): String =
    val c: Char = Elems.first("abc")
    c.toString
