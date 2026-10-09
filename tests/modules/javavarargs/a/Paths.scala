package jva

import java.nio.file.{Files, Path, Paths}

// The JDK's file members, which the lean std declares without their Java varargs (`LinkOption*`,
// `String*`): the products' calls pass them empty, as scalac's.
object Probe:
  def describe(s: String): String =
    val path = Path.of(s)
    val kind =
      if Files.isDirectory(path) then "dir"
      else if Files.isRegularFile(path) then "file"
      else if Files.exists(path) then "other"
      else "none"
    val dated = Files.exists(Paths.get(s)) && Files.getLastModifiedTime(path).toMillis > 0
    s"$kind $dated"
