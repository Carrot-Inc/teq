// `String.startsWith(prefix, offset)`, Java's, which scala-library's `NameTransformer.decode` calls.
@main def main(): Unit =
  println("abcde".startsWith("cd", 2))
  println("abcde".startsWith("cd", 1))
  println("abcde".startsWith("", 5))
  println("abcde".startsWith("", 6))
  println("abcde".startsWith("a", -1))
  println("abcde".startsWith("de", 3))
