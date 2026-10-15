// A val named by an operator, as tyrian-next's `syntax` re-exports `Batch.==:` and `Batch.:==`:
// dotty's `simplePattern` takes any identifier for the name, one ending in a colon too.
object Ops:
  def ==:(x: Int): Int = x + 1
  def :==(x: Int): Int = x + 2

object syntax:
  val ==: = Ops.==:
  val :== = Ops.:==
  val +++ : Int = 3
  var <> = "v"

@main def run(): Unit =
  import syntax.*
  println(==:(1) + :==(1) + +++)
  <> = "w"
  println(<>)
