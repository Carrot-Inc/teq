//> using platform jvm
// The JDK members play's routes compiler reads, as the interpreter has them: `File.toURI` (`file:`
// and the absolute path, a directory's ending in `/`), `File.createNewFile`, `Character.
// isJavaIdentifierStart` and a `WeakHashMap` (scala-parser-combinators' cache).
import java.io.File

@main def run(): Unit =
  val dir = java.nio.file.Files.createTempDirectory("routes").toFile
  val f = new File(dir, "Routes.scala")
  println(f.toURI.toString.startsWith("file:/") && f.toURI.toString.endsWith("/Routes.scala"))
  println(dir.toURI.toString.endsWith("/"))
  println(f.createNewFile())
  println(f.createNewFile())
  println(f.exists && f.length == 0)
  println(f.delete() && dir.delete())
  println(List('a', '_', '$', '1', '-', 'é').map(Character.isJavaIdentifierStart))
  val cache = new java.util.WeakHashMap[String, Int]()
  cache.put("x", 1)
  println(cache.get("x"))
  println(cache.containsKey("y"))
