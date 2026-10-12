import E.*
@main def main() =
  println(Mac.rebuilt(1.plus(2)(3)))
  println(Mac.copied(1.plus(2)(3)))
  println(Mac.rebuilt(1 -> 2))
  println(Mac.copied("a" -> 2.5))
