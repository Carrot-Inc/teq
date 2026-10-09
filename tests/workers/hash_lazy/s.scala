// An object whose lazy val one run reads between another run's identity hash of it and a third's:
// the slot is left out of the hash whether or not it was read,
// so a worker that reads it before its first hash gives the hash the run in the prefix gave.
object S:
  var n = 3
  lazy val x: Int = n * 2
