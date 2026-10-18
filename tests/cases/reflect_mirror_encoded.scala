// The mirror class of a top-level object whose name is no identifier, found by the object's run-time name without
// its `$`: the target's one name for it (dotty's mirrorClassBTypeFromSymbol, javaBinaryName.stripSuffix("$"), the
// JVM's encoded name under teq interp and on the JVM, Scala.js's written one on JavaScript), as scalac's is.
object `A-B`
object Main:
  def main(args: Array[String]): Unit =
    val name = `A-B`.getClass.getName.stripSuffix("$")
    try
      Class.forName(name)
      println("found")
    catch
      case _: ClassNotFoundException => println("missing")
