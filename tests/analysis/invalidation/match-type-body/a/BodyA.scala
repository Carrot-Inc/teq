package mta

// The match type reduced in a body alone, as tests/modules/matchtype's use: the signature
// names neither `Elem` nor its file.
object BodyA:
  def show(): String =
    val c = Elems.first("abc")
    c.toString
