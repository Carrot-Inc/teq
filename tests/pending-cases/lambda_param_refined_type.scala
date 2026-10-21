// A function literal's parameter typed with a refinement or a match type (`(i: Box { val y: 1 }) => ..`,
// `(i: T match { .. }) => ..`), which teq's parser reads as an expression and refuses ("expected end of statement").
// Met by the probes of the review of `16412b2e9` (a polymorphic function literal's evidence before such a parameter).
type Elem[T] = T match { case List[e] => e }
trait Box { val y: Int }
object Test {
  def main(args: Array[String]): Unit = {
    val refined = (i: Box { val y: 1 }) => i.y + 1
    val matched = (i: Elem[List[Int]] match { case Int => String }) => i.length
    println(refined(new Box { val y: 1 = 1 }))
    println(matched("two"))
  }
}
