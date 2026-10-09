//> using platform jvm
// A generator of sources as a build runs one: for an application directory, a Scala object per
// image folder with a val per `.svg` file under it, written into the output directory, and a
// JavaScript module naming the images, written into the application's own assets; each file
// written only when its text changes, so that a build that runs it again finds its sources as they
// were.
import java.nio.file.{Files, Path}
import scala.jdk.CollectionConverters.*

object Generate:
  val folders = List("shapes" -> "ShapeImages", "marks" -> "MarkImages")

  def run(app: Path, out: Path, written: String => Unit): Unit =
    val images = app.resolve("assets/images")
    val found = folders.map((folder, name) => (folder, name, svgs(images.resolve(folder))))
    writeIfChanged(app.resolve("assets/images.js"), module(found), written)
    for (folder, name, files) <- found do
      writeIfChanged(out.resolve("images").resolve(name + ".scala"), scalaObject(folder, name, files), written)

  // The `.svg` files under the folder, as paths relative to it with `/` between their names.
  def svgs(dir: Path): List[String] =
    if !Files.isDirectory(dir) then Nil
    else
      val walk = Files.walk(dir)
      try
        walk.iterator.asScala.toList
          .filter(p => Files.isRegularFile(p) && p.getFileName.toString.endsWith(".svg"))
          .map(p => dir.relativize(p).toString.replace('\\', '/').stripSuffix(".svg"))
          .sorted
      finally walk.close()

  def identifier(file: String): String =
    val parts = file.split('/')
    val name = parts.head + parts.tail.map(_.capitalize).mkString
    require(name.matches("[a-z][a-zA-Z0-9]*"), s"an image's name is a Scala identifier: $file.svg")
    name

  def module(found: List[(String, String, List[String])]): String =
    val sections = for (folder, _, files) <- found yield
      val entries = files.map(f => s"""  ${identifier(f)}: "/assets/images/$folder/$f.svg",""").mkString("\n")
      s"export const $folder = {\n$entries\n};\n"
    "// Written by the image generator — do not edit.\n\n" + sections.mkString("\n")

  def scalaObject(folder: String, name: String, files: List[String]): String =
    val vals = files.map(f => s"""  val ${identifier(f)}: String = "/assets/images/$folder/$f.svg"""").mkString("\n")
    s"""package images
       |
       |// Written by the image generator from assets/images/$folder — do not edit.
       |object $name:
       |$vals
       |""".stripMargin

  def writeIfChanged(file: Path, content: String, written: String => Unit): Unit =
    if !Files.exists(file) || Files.readString(file) != content then
      Files.createDirectories(file.getParent)
      Files.writeString(file, content)
      written(file.getFileName.toString)
