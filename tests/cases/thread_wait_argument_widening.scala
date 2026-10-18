//> using platform jvm
// Object.wait's arguments widened as an application's are: an Int, a Byte and a Char where wait takes a Long, a
// Byte where wait(long, int) takes an Int (dotty's FunProto.typedArg adapting each to its parameter's type).
object Main:
  def main(args: Array[String]): Unit =
    val lock = new Object
    val b: Byte = 1
    lock.synchronized { lock.wait(1); lock.wait(1, 0); lock.wait(b); lock.wait(2L, b); lock.wait('c'); lock.notify() }
    println("ok")
