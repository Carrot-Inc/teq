// A given imported in a block of the same method as a local given of the same name: scalac
// counts the import at the level of the scope around it (`ContextualImplicits.level`: an import
// of the same owner starts no level) and keeps the definition, which beats an import of the
// name at one level (`combineEligibles`); teq takes the import, as nearer. The same holds of
// an import in an inline body expanded next to a given of the call site. scalac prints 9, 5, 9.
import scala.compiletime.summonInline
object Values:
  given Int = 7
inline def imported: Int =
  import Values.given
  summonInline[Int]
@main def run(): Unit =
  given Int = 9
  locally {
    import Values.given
    println(summon[Int])
  }
  locally {
    given Int = 5
    locally {
      import Values.given
      println(summon[Int])
    }
  }
  println(imported)
