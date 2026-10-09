package cpa

// Constants read from the products: each is its literal downstream, as the whole build folds
// the program's own and scalac folds a pickle's, so the object is not initialised for them.
object Consts:
  println("Consts initialised")
  final val K = 1
  final val T: 2 = 2
  inline val I = 3
  final val S = "s"
  final val D = 0.5
  final val L = 4L
  final val B = true
  final val C = 'c'
  final lazy val Z: 7 = 7
  final implicit val J: 8 = 8
  val notConstant: 5 = 5
