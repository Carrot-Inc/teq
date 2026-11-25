// A macro's `try` expanded as a List and a Vector literal's element: the literal's `new ArraySeq$ofRef` stands
// uninitialised under it, and the try's stash holds it at its own type.
object Main:
  def main(args: Array[String]): Unit =
    val n = args.length + 3
    println(List(M.caught(n), n * 2 + 5))
    println(Vector(n * 2 + 5, M.caught(n + 200)))
