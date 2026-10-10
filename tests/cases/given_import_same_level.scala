// A given imported in a block of the same method as a local given of the same name: the import
// stands at the level of the scope around it (`ContextualImplicits.level`: an import of the same
// owner starts no level), where the definition beats an import of the name (`combineEligibles`).
// The same holds of an import in an inline body expanded next to a given of the call site.
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
