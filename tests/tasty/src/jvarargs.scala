package fix.jvarargs

import java.nio.file.{Files, Path, Paths}

// Java varargs members the lean std declares without their varargs: scala-library's code passes
// them an empty array, as the calls scalac writes do too.
object JVarargs:
  def exists(p: Path): Boolean = Files.exists(p)
  def dir(p: Path): Boolean = Files.isDirectory(p)
  def regular(p: Path): Boolean = Files.isRegularFile(p)
  def modified(p: Path): Long = Files.getLastModifiedTime(p).toMillis
  def of(s: String): Path = Path.of(s)
  def get(s: String): Path = Paths.get(s)
