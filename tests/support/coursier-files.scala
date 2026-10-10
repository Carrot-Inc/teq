//> using scala 3.8.4
//> using dep io.get-coursier:coursier-paths:2.1.25-M26
// The golden cases of tests/support/coursier-files.txt from coursier itself: the second column of each case, the
// file coursier's cache keeps for the URL of the first, as coursier-paths 2.1.25-M26 (the coursier sbt 2.0.8 embeds)
// names it, CachePath.localFile under a cache directory with no user, relative to that directory with `/`; `-` for a
// URL coursier refuses. Rewrites the file's case lines, comments left as they are; `--check` writes nothing and
// exits 1 naming each line that differs. It calls coursier's own class, so it runs on a JVM, never under teq: from
// the repository's root, `scala-cli run tests/support/coursier-files.scala --server=false [-- --check]`.
import java.io.File
import java.nio.file.{Files, Paths}

@main def coursierFiles(args: String*): Unit =
  val check = args == Seq("--check")
  if !check && args.nonEmpty then
    System.err.println("usage: scala-cli run tests/support/coursier-files.scala [-- --check]")
    sys.exit(2)
  val table = Paths.get("tests/support/coursier-files.txt")
  val cache = new File("/c")
  val lines = Files.readAllLines(table).toArray.map(_.toString).toVector
  val written = lines.map { line =>
    if line.isEmpty || line.startsWith("#") then line
    else
      val url = line.takeWhile(_ != ' ')
      val file =
        try cache.toPath.relativize(coursier.paths.CachePath.localFile(url, cache, null, false).toPath).toString.replace('\\', '/')
        catch case _: IllegalArgumentException | _: java.net.MalformedURLException => "-"
      s"$url $file"
  }
  val differing = lines.zip(written).filter(_ != _)
  if check then
    for (was, is) <- differing do System.err.println(s"coursier-files: `$was` is `$is` by coursier")
    println(s"coursier-files: ${written.count(l => l.nonEmpty && !l.startsWith("#"))} cases, ${differing.size} differing from coursier-paths 2.1.25-M26")
    sys.exit(if differing.isEmpty then 0 else 1)
  Files.write(table, written.mkString("", "\n", "\n").getBytes("UTF-8"))
  println(s"coursier-files: ${differing.size} lines rewritten by coursier-paths 2.1.25-M26")
