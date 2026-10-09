//> using platform jvm
// The streams of `Files.list` and `Files.walk` through the stream operations: their size is not
// known, so `count` runs the stages; the stages are lazy and the traversal stops where a stage
// does; closing any stage of the pipeline closes the directory, once. Its files are under
// target/ of the working directory, removed at the end.
import java.nio.file.{Files, Path, Paths}
import java.util.stream.Collectors

object Main:
  def show(name: String)(f: => Any): Unit =
    try println(name + ": " + f)
    catch case e: Exception => println(name + ": " + e.getClass.getName + ": " + e.getMessage)

  def remove(p: Path): Unit =
    if Files.exists(p) then
      val s = Files.walk(p)
      val all = try s.map[String](_.toString).sorted((a, b) => b.compareTo(a)).collect(Collectors.toList[String]()) finally s.close()
      all.forEach(x => Files.deleteIfExists(Paths.get(x)))

  def main(args: Array[String]): Unit =
    val root = Paths.get("target/files_stream_ops")
    remove(root)
    Files.createDirectories(root.resolve("a/b"))
    Files.writeString(root.resolve("a/one.txt"), "1")
    Files.writeString(root.resolve("a/two.md"), "22")
    Files.writeString(root.resolve("a/b/three.txt"), "333")
    try run(root)
    finally remove(root)

  def run(root: Path): Unit =
    def rel(p: Path): String = root.relativize(p).toString
    show("walk sorted names")(Files.walk(root).map[String](rel).sorted().collect(Collectors.toList[String]()))
    show("walk files by length")({
      val s = Files.walk(root)
      try s.filter(p => Files.isRegularFile(p)).mapToInt(p => Files.readString(p).length).sum()
      finally s.close()
    })
    show("list count runs")({
      var seen = 0
      val s = Files.list(root.resolve("a"))
      try s"${s.peek(_ => seen += 1).count()} after $seen"
      finally s.close()
    })
    show("grouped by extension")({
      val s = Files.walk(root)
      try
        val names = s.filter(p => Files.isRegularFile(p)).map[String](p => p.getFileName.toString).sorted()
        val grouped = names.collect(Collectors.groupingBy[String, String, java.util.List[String], Any, java.util.TreeMap[String, java.util.List[String]]](
          n => n.substring(n.indexOf('.') + 1), () => new java.util.TreeMap[String, java.util.List[String]](),
          Collectors.toList[String]().asInstanceOf[java.util.stream.Collector[String, Any, java.util.List[String]]]))
        grouped
      finally s.close()
    })
    show("anyMatch stops the walk")({
      val events = new StringBuilder
      val s = Files.walk(root).onClose(() => events.append("[closed]"))
      val found =
        try s.map[String](rel).peek(n => events.append(if n.isEmpty then "." else n).append(' ')).anyMatch(_ == "a")
        finally s.close()
      s"$found $events"
    })
    show("close through a stage")({
      val events = new StringBuilder
      val s = Files.list(root.resolve("a")).onClose(() => events.append("[closed]"))
      val names = s.map[String](p => p.getFileName.toString).onClose(() => events.append("[stage]"))
      names.close()
      names.close()
      events.toString
    })
    show("source after a stage")({
      val s = Files.list(root)
      s.filter(_ => true)
      try s.count() finally s.close()
    })
    show("iterator of a stage")({
      val s = Files.list(root.resolve("a")).filter(p => Files.isDirectory(p)).map[String](p => p.getFileName.toString)
      try
        val it = s.iterator()
        it.next() + " " + it.hasNext
      finally s.close()
    })
