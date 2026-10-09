package oea

// A library's transparent method with an added overload, whose expansions the middle module's
// pickles hold: the downstream's reader names the callee by its signature (kind 8), and the
// outlining of the repeated expansions is the whole build's.
object Fmt:
  transparent inline def show[T](inline label: String, x: Int, y: T): String =
    val pre = label + ": "
    val k = pre.length + x
    pre + x.toString + (if k > 10 then "!" else ".") + k.toString + " " + y.toString

  transparent inline def show(inline label: String, x: Long): String =
    val pre = label + "; "
    val k = pre.length + x
    pre + x.toString + (if k > 10L then "!" else ".") + k.toString
