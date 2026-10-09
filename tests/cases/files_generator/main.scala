// The generator over a copy of the fixtures under target/ of the working directory, removed at the
// end: its first run writes the module and both objects, a second writes nothing, an image added
// rewrites its folder's object and the module, the module deleted alone is written again; the
// files it wrote are printed whole, with their length in bytes.
import java.nio.file.{Files, Path, Paths}
import scala.jdk.CollectionConverters.*

object Main:
  def remove(p: Path): Unit =
    if Files.exists(p) then
      val s = Files.walk(p)
      val all = try s.iterator.asScala.toList finally s.close()
      all.reverse.foreach(Files.deleteIfExists)

  def copy(from: Path, to: Path): Unit =
    val s = Files.walk(from)
    try
      for p <- s.iterator.asScala do
        val target = to.resolve(from.relativize(p).toString)
        if Files.isDirectory(p) then Files.createDirectories(target)
        else Files.write(target, Files.readAllBytes(p))
    finally s.close()

  def generate(app: Path, out: Path, what: String): Unit =
    val written = List.newBuilder[String]
    Generate.run(app, out, written += _)
    println(what + ": " + written.result().sorted.mkString(", "))

  def main(args: Array[String]): Unit =
    val root = Paths.get("target/files_generator")
    remove(root)
    try
      val app = root.resolve("app")
      val out = root.resolve("out")
      copy(Paths.get("tests/cases/files_generator/fixtures"), app.resolve("assets/images"))
      generate(app, out, "first run")
      generate(app, out, "second run")
      Files.writeString(app.resolve("assets/images/marks/cross.svg"), "<svg/>\n")
      generate(app, out, "an image added")
      Files.deleteIfExists(app.resolve("assets/images.js"))
      generate(app, out, "the module deleted")
      for f <- List(app.resolve("assets/images.js"), out.resolve("images/ShapeImages.scala"), out.resolve("images/MarkImages.scala")) do
        println("== " + root.relativize(f) + " (" + Files.readAllBytes(f).length + " bytes)")
        print(Files.readString(f))
    finally remove(root)
