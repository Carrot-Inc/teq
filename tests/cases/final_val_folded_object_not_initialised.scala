// A final val of no written type that overrides nothing and whose right-hand side folds is a constant: a use reads
// the constant and does not initialise its object, as scalac folds it (the constant type Namer keeps for a final or
// inline val); one that does not fold is read through the object, which is then initialised.
object O {
  println("O initialised")
  final val shift = 1 << 6
  final val text = "a" + "b"
  inline val sum = 2 + 3
  final val read = shift.toString.length
}
@main def main(): Unit =
  println(O.shift)
  println(O.text)
  println(O.sum)
  println(O.read)
