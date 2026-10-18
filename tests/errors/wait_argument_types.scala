// Object.wait's arguments conform as an application's do (dotty's Applications.addArg, FunProto.typedArg): a
// String where wait(long) and wait(long, int) take a Long and an Int is refused at the argument, as scalac
// 3.8.4 refuses it ("Found: ("bad" : String)" with "Required: Long", then "Required: Int").
object Main:
  def main(args: Array[String]): Unit =
    val lock = new Object
    lock.synchronized { lock.wait("bad"); lock.wait(1L, "bad") }
// expect: 7:35: error: type mismatch: found "bad", required Long
// expect: 7:57: error: type mismatch: found "bad", required Int
// expect: 2 errors found
