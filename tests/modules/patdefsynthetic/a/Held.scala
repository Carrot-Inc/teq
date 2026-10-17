package pds

// A pattern definition's holder is `Synthetic` (`Desugar.makePatDef`'s `patMods`) in the pickle,
// whichever compiler wrote it: an object's strict and lazy holders, and the local one of an inline
// method's body, which the downstream's expansion copies.
object Held:
  def flag: Boolean = System.nanoTime() != 0L
  val (x, y) = if flag then (1, 2) else (3, 4)
  lazy val (u, v) = if flag then (5, 6) else (7, 8)
  inline def sum(f: Boolean): Int =
    val (a, b) = if f then (1, 2) else (3, 4)
    a + b
