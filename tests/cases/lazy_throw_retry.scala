// A lazy val whose initialiser throws is not computed: the exception reaches the reader and the next read runs the
// initialiser again (dotty's LazyVals: the state reset in the init's finally for a member, the holder left
// uninitialised under its monitor for a local), for members of a class and an object, a local, and an initialiser
// of each erased kind.
class Flaky:
  var tries = 0
  lazy val ref: String = { tries += 1; if tries == 1 then throw new IllegalStateException("ref"); s"ref after $tries" }
  lazy val int: Int = { tries += 1; if tries < 4 then throw new IllegalStateException("int"); tries * 10 }
  lazy val unit: Unit = { tries += 1; if tries < 6 then throw new IllegalStateException("unit"); println(s"unit after $tries") }

object Once:
  var tries = 0
  lazy val value: Long = { tries += 1; if tries == 1 then throw new IllegalStateException("object"); tries.toLong << 33 }

object Main:
  def attempt[T](read: => T): String =
    try read.toString catch case e: IllegalStateException => "threw " + e.getMessage

  def main(args: Array[String]): Unit =
    val f = new Flaky
    println(List(attempt(f.ref), attempt(f.ref), attempt(f.ref)))
    println(List(attempt(f.int), attempt(f.int), attempt(f.int)))
    println(List(attempt(f.unit), attempt(f.unit), attempt(f.unit)))
    println(List(attempt(Once.value), attempt(Once.value)))
    var localTries = 0
    lazy val local: Double = { localTries += 1; if localTries == 1 then throw new IllegalStateException("local"); localTries * 1.25 }
    println(List(attempt(local), attempt(local), attempt(local)))
