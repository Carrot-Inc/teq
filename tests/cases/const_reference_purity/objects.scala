// A final val's selection on an object that initialises is its literal (an inline val needs an idempotent
// expression only), the object uninitialised; a non-final constant val's selection there is no pure expression and
// stays, so the object runs.
object O:
  val y: 42 = { println("init O"); 42 }
  final val w = 7
object Q:
  println("init Q")
  val v: 42 = 42
