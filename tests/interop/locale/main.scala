import java.util.Locale

@main def main(): Unit =
  println("Title".toUpperCase(Locale.US))
  println("Title".toLowerCase(Locale.ROOT))
  println(List("a", "b").map(_.toUpperCase(Locale.US)).mkString(","))
  println("Title".toLowerCase + "Title".toUpperCase())
