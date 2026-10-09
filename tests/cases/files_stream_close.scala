//> using platform jvm
// The streams of `Files.list` and `Files.walk` are the caller's to close, as the JDK's are: neither
// the stream's `toList` nor a Scala iterator's `toList` closes one, its `onClose` handlers run at
// `close`, once, in their order, whether the traversal ended or an exception left it; a stream is
// used once, and its iterator fails once the walk is closed. Its files are under target/ of the
// working directory, removed at the end.
import java.nio.file.{Files, Path, Paths}
import scala.jdk.CollectionConverters.*

object Main:
  def show(name: String)(f: => Any): Unit =
    try println(name + ": " + f)
    catch case e: Exception => println(name + ": " + e.getClass.getName + ": " + e.getMessage)

  def remove(p: Path): Unit =
    if Files.exists(p) then
      val s = Files.walk(p)
      val all = try s.iterator.asScala.toList finally s.close()
      all.reverse.foreach(Files.deleteIfExists)

  def main(args: Array[String]): Unit =
    val root = Paths.get("target/files_stream_close")
    remove(root)
    Files.createDirectories(root.resolve("a/b/c"))
    Files.writeString(root.resolve("a/b/c/leaf.txt"), "leaf")
    try run(root)
    finally remove(root)

  def run(root: Path): Unit =
    val events = new StringBuilder
    // A traversal to its end: the handler runs at close, not before.
    val walk = Files.walk(root.resolve("a")).onClose(() => events.append("[closed]"))
    try
      for p <- walk.iterator.asScala.toList do events.append(root.relativize(p)).append(' ')
      events.append("[traversed] ")
    finally walk.close()
    println("walk: " + events)
    // An exception in the middle of the traversal: the handler still runs, through the finally.
    events.clear()
    show("exception mid-traversal") {
      val s = Files.walk(root.resolve("a")).onClose(() => events.append("[closed]"))
      try s.iterator.asScala.map(p => if p.getFileName.toString == "c" then throw new IllegalStateException("at " + root.relativize(p)) else p).toList
      finally s.close()
    }
    println("after the exception: " + events)
    // The stream's toList closes nothing; the handlers run once, in their order.
    events.clear()
    val listed = Files.list(root.resolve("a")).onClose(() => events.append("1")).onClose(() => events.append("2"))
    show("toList")(listed.toList.asScala.map(p => root.relativize(p).toString))
    show("closed after toList")(events.toString)
    listed.close()
    listed.close()
    show("closed after close twice")(events.toString)
    show("toList after close")(listed.toList)
    show("iterator after close")(listed.iterator)
    show("onClose after close")(listed.onClose(() => ()))
    // A stream is used once.
    val once = Files.walk(root)
    val it = once.iterator
    show("iterator again")(once.iterator)
    show("toList after iterator")(once.toList)
    show("first")(root.relativize(it.next()).toString.isEmpty)
    once.close()
    show("walk iterator after close")(it.hasNext)
    val list = Files.list(root.resolve("a"))
    val listIt = list.iterator
    list.close()
    show("list iterator after close")(listIt.hasNext)
    // An element `hasNext` took ahead is the iterator's still once the stream is closed.
    for (kind, s) <- List("walk" -> Files.walk(root.resolve("a")), "list" -> Files.list(root.resolve("a"))) do
      val i = s.iterator
      show(kind + ": hasNext before close")(i.hasNext)
      s.close()
      show(kind + ": next after close")(root.relativize(i.next()))
      show(kind + ": hasNext after that")(i.hasNext)
    // To the end of a walk and past it.
    val ended = Files.walk(root.resolve("a/b/c"))
    try
      val i = ended.iterator
      while i.hasNext do i.next()
      show("next past the end")(i.next())
    finally ended.close()
    // A handler that throws: the first exception after every handler ran.
    events.clear()
    val throwing = java.util.stream.Stream.of("x").onClose(() => throw new IllegalStateException("first")).onClose(() => { events.append("second ran"); throw new IllegalArgumentException("second") })
    show("throwing handlers")(throwing.close())
    show("after them")(events.toString)
    show("Stream.of")(java.util.stream.Stream.of("x", "y").toList)
    show("count")(java.util.stream.Stream.of("x", "y", "z").count())
    show("toList is unmodifiable")(java.util.stream.Stream.of("x").toList.add("y"))
    // A null consumer is refused on a stream with no element too, and the stream is used.
    val none = java.util.stream.Stream.empty[String]()
    show("forEach null")(none.forEach(null))
    show("count after it")(none.count())
