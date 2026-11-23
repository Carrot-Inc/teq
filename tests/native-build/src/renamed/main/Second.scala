package renamed

// Reaches A as well, whose directory sorts before AaBB and takes the hash's name from it.
object Second {
  def main(args: Array[String]): Unit =
    println(A.value(args.length) + B.value)
}
