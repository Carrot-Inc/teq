// The part of os-lib (com.lihaoyi::os-lib) tyrian's generators call, for bench/tyrian/overlay/generate.scala:
// a path, `/`, `exists` and `write` (which, as os-lib's, refuses a file that exists), over java.nio.file.
// os-lib itself does not run under `teq interp` yet: its `write` reaches JavaConverters and file channels the
// lean std lacks.
package os

import java.nio.file.{Files, Paths, StandardOpenOption}

final class Path(val wrapped: java.nio.file.Path):
  def /(segment: String): Path = Path(wrapped.resolve(segment))
  override def toString: String = wrapped.toString

object Path:
  def apply(path: java.nio.file.Path): Path = new Path(path)
  def apply(path: String): Path = new Path(Paths.get(path).toAbsolutePath.normalize)

def exists(path: Path): Boolean = Files.exists(path.wrapped)

def write(path: Path, data: String): Unit = Files.writeString(path.wrapped, data, StandardOpenOption.CREATE_NEW)
