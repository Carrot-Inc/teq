package xa

// Inline methods whose bodies match tuples of more than 22 elements: the published pattern keeps
// each element's type (`x: Int`, `s: String`, `l: Long`) for the downstream module that inlines them.
object Up:
  inline def last(t: (Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int)): Int =
    t match
      case (_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, x) => x
  inline def mixed(t: (Int, String, Int, String, Int, String, Int, String, Int, String, Int, String, Int, String, Int, String, Int, String, Int, String, Int, String, Long)): String =
    t match
      case (n, s, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, l) => s.toUpperCase + (n + 1) + (l / 2)
