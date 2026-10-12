//> using platform jvm
// split-watch: jvm
// A std object standing for a JDK class takes the class's statics it leaves out when an import names
// them (`import java.lang.Character.*` then `getType`), as when a selection does: lila's
// `String.isShouting`. A JVM build's, whose JDK has them.
object Shout:
  def isShouting(text: String): Boolean =
    text.lengthIs >= 5 && {
      import java.lang.Character.*
      text.foldLeft(0) { (i, c) =>
        getType(c) match
          case UPPERCASE_LETTER => i + 1
          case LOWERCASE_LETTER => i - 1
          case _ => i
      } > 0
    }

@main def run(): Unit =
  println(Shout.isShouting("HELLO you"))
  println(Shout.isShouting("hello THERE friends"))
