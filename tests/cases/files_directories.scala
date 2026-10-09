//> using platform jvm
// The directories of `java.nio.file.Files` against the JDK: `createDirectory`, `createDirectories`
// (a directory that is there is fine, a file in its place is a `FileAlreadyExistsException`, the
// path answered as given), `deleteIfExists`, `list` and `walk` (the start first, then depth first,
// a file's walk the file alone, a depth bound) and the predicates, on relative and absolute paths
// and paths that are not there. Its files are under target/ of the working directory, removed at
// the end; a directory with more than one entry is listed sorted, as the system's order is its own.
import java.nio.file.{Files, Path, Paths}
import scala.jdk.CollectionConverters.*

object Main:
  val dir = System.getProperty("user.dir")
  def shown(text: String): String = if text == null then "null" else text.replace(dir, "<dir>")

  def show(name: String)(f: => Any): Unit =
    try println(name + ": " + shown(String.valueOf(f)))
    catch case e: Exception => println(name + ": " + e.getClass.getName + ": " + shown(e.getMessage))

  def remove(p: Path): Unit =
    if Files.exists(p) then
      val s = Files.walk(p)
      val all = try s.iterator.asScala.toList finally s.close()
      all.reverse.foreach(Files.deleteIfExists)

  def walked(p: Path, depth: Int = Int.MaxValue): String =
    val s = Files.walk(p, depth)
    try s.iterator.asScala.map(e => shown(e.toString)).mkString(", ")
    finally s.close()

  def listed(p: Path): String =
    val s = Files.list(p)
    try s.iterator.asScala.map(e => shown(e.toString)).toList.sorted.mkString(", ")
    finally s.close()

  def main(args: Array[String]): Unit =
    val root = Paths.get("target/files_directories")
    remove(root)
    Files.createDirectories(root)
    try run(root)
    finally remove(root)

  def run(root: Path): Unit =
    val file = root.resolve("file.txt")
    Files.writeString(file, "text")
    val absent = root.resolve("absent")
    for (kind, p) <- List("file" -> file, "directory" -> root, "absent" -> absent, "absolute file" -> file.toAbsolutePath, "absolute absent" -> absent.toAbsolutePath) do
      show("exists " + kind)(Files.exists(p))
      show("isDirectory " + kind)(Files.isDirectory(p))
      show("isRegularFile " + kind)(Files.isRegularFile(p))
    // The empty path is the working directory, the repository's root.
    show("exists the empty path")(Files.exists(Paths.get("")))
    show("the empty path lists the working directory")({ val s = Files.list(Paths.get("")); try s.iterator.asScala.exists(_.toString == "tests") finally s.close() })
    // createDirectories and createDirectory
    val nested = root.resolve("a/b/c")
    show("createDirectories nested")(Files.createDirectories(nested))
    show("createDirectories again")(Files.createDirectories(nested) eq nested)
    show("createDirectories absolute")(Files.createDirectories(root.resolve("abs/x").toAbsolutePath))
    show("createDirectories dots")(Files.createDirectories(root.resolve("d/./e/../f")))
    show("createDirectories dots made")(listed(root.resolve("d")))
    show("createDirectories file")(Files.createDirectories(file))
    show("createDirectories under a file")(Files.createDirectories(file.resolve("x/y")))
    show("createDirectory")(Files.createDirectory(root.resolve("single")))
    show("createDirectory there")(Files.createDirectory(root.resolve("single")))
    show("createDirectory absent parent")(Files.createDirectory(root.resolve("none/single")))
    // list
    show("list")(listed(root))
    show("list nested")(listed(root.resolve("a")))
    show("list absolute")(listed(root.resolve("a").toAbsolutePath))
    show("list file")(Files.list(file))
    show("list absent")(Files.list(absent))
    show("list empty")(listed(root.resolve("single")))
    // walk: the start, then each directory's entries after it
    show("walk")(walked(root.resolve("a")))
    show("walk absolute")(walked(root.resolve("a").toAbsolutePath))
    show("walk file")(walked(file))
    show("walk absent")(Files.walk(absent))
    show("walk depth 0")(walked(root.resolve("a"), 0))
    show("walk depth 1")(walked(root.resolve("a"), 1))
    show("walk negative depth")(Files.walk(root, -1))
    show("walk empty")(walked(root.resolve("single")))
    show("walk all")({ val s = Files.walk(root); try s.iterator.asScala.map(e => root.relativize(e).toString).toList.sorted.mkString(", ") finally s.close() })
    // deleteIfExists: a file, an empty directory, nothing, a directory with entries
    show("deleteIfExists file")(Files.deleteIfExists(file))
    show("deleteIfExists again")(Files.deleteIfExists(file))
    show("deleteIfExists empty directory")(Files.deleteIfExists(root.resolve("single")))
    show("deleteIfExists directory with entries")(Files.deleteIfExists(root.resolve("a")))
    show("deleteIfExists absolute")(Files.deleteIfExists(root.resolve("a/b/c").toAbsolutePath))
    show("deleteIfExists under an absent directory")(Files.deleteIfExists(absent.resolve("x")))
    show("left")(listed(root))
