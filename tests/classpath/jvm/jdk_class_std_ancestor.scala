// jars: scala-library
// std: lean scala-library
// A JDK class nothing has read yet against a bound the std stands in for: `FileLock`, which
// `FileChannel.lock()` returns, is an `AutoCloseable` (the std's `java.lang.AutoCloseable`), so
// `from(blk(ch.lock()))` infers `A = FileLock` under `A <: AutoCloseable`.
import java.nio.channels.FileChannel
import java.nio.file.{Files, StandardOpenOption}

final class Box[+A](a: => A):
  def get: A = a

def from[A <: AutoCloseable](fa: => Box[A]): Box[A] = fa
def blk[A](a: => A): Box[A] = new Box(a)

object Main:
  def main(args: Array[String]): Unit =
    val path = Files.createTempFile("teq", ".lock")
    val ch = FileChannel.open(path, StandardOpenOption.WRITE)
    val lock = from(blk(ch.lock())).get
    println(lock.isValid)
    lock.close()
    println(lock.isValid)
    ch.close()
    Files.delete(path)
