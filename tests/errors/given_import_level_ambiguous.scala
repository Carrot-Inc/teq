// expect: 9:53: error: ambiguous given instances for Int: given_Int, nine
// expect: 1 error found
// An import of the method's owner stands at the level of the method's givens (`ContextualImplicits.level`), so a
// given of another name there and the imported one are both candidates of one level.
object Values:
  given Int = 7
@main def run(): Unit =
  given nine: Int = 9
  locally { import Values.given; println(summon[Int]) }
