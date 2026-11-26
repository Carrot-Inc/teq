// A constant val whose type is written as a singleton path (a library constant's, a final val's, a Java static's) is
// read as its constant from another file, as scalac folds the read: the object that holds it is not initialised by the
// read, whichever worker types its initialiser.
object O:
  println("init")
  final val t = true
  inline val b: Int.MaxValue.type = Int.MaxValue
  inline val c: t.type = t
  inline val j: java.lang.Integer.MAX_VALUE.type = java.lang.Integer.MAX_VALUE
  final val h: t.type = t
  final val k: Long.MaxValue.type = Long.MaxValue
